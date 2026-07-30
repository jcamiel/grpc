/*
 * Hurl (https://hurl.dev)
 * Copyright (C) 2026 Orange
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *          http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */
use std::fmt;
use std::fmt::Formatter;

use base64::Engine;
use base64::alphabet;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use serde_json::{Map, Number, Value};

use crate::schema::descriptor::{DescriptorProto, FieldDescriptorProto, FieldLabel, FieldType};
use crate::schema::symbols::SymbolTable;
use crate::wire::writer::Writer;

#[derive(Debug)]
pub struct Field {
    kind: FieldKind,
    number: u32,
}

impl Field {
    pub fn number(&self) -> u32 {
        self.number
    }
}

#[derive(Debug)]
pub enum FieldKind {
    /// A string field
    String(String),
    /// A boolean field
    Bool(bool),
    /// A repeated field
    Array(Vec<Field>),
    /// A message field
    Message(Vec<Field>),
    /// All signed int32 fields
    SFixed32(i32),
    Int32(i32),
    SInt32(i32),
    /// All unsigned uint32 fields
    UInt32(u32),
    Fixed32(u32),
    /// All signed int64 fields
    SFixed64(i64),
    Int64(i64),
    SInt64(i64),
    /// All unsigned uint64 fields
    UInt64(u64),
    Fixed64(u64),
    /// All floating-point fields
    Double(f64),
    Float(f32),
    /// A bytes field
    Bytes(Vec<u8>),
}

#[derive(Debug)]
pub enum FieldError {
    /// The JSON input type doesn't match the expected type given the actual descripor
    InvalidJsonInputType {
        field: String,
        expected: String,
        actual: String,
    },
    /// The symbol table (or the descriptor) doesn't know the type `type_name` of a field named `field`.
    UnresolvedType { field: String, type_name: String },
    /// The input JSON has a `field` which is not present in the type name `type_name`.
    UnknownJsonField { field: String, type_name: String },
    /// The JSON is a number but its value is out of the target field's numeric range.
    JsonNumberOutOfRange { field: String, value: String },
    /// A string is used for representing integer, but this string is not parseable as an integer
    InvalidStringAsInteger { field: String, value: String },
    /// The JSON value is a string but not valid base64.
    InvalidBase64 { field: String, value: String },
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            FieldError::InvalidJsonInputType {
                field,
                expected,
                actual,
            } => write!(
                f,
                "expecting {expected} for field '{field}' while actual is {actual}"
            ),
            FieldError::UnresolvedType { field, type_name } => {
                write!(f, "type '{type_name}' is unknown for field '{field}'")
            }
            FieldError::UnknownJsonField { field, type_name } => write!(
                f,
                "message type '{type_name}' has no known field named '{field}'"
            ),
            FieldError::JsonNumberOutOfRange { field, value } => {
                write!(f, "number '{value}' is out of range for field '{field}'")
            }
            FieldError::InvalidStringAsInteger { field, value } => {
                write!(f, "parsing '{value}' as integer failed for field '{field}'")
            }
            FieldError::InvalidBase64 { field, value } => write!(
                f,
                "'{value}' is not a valid base64 string for field '{field}'"
            ),
        }
    }
}

impl Field {
    pub fn try_new(
        descriptor: &FieldDescriptorProto,
        symbols: &SymbolTable,
        value: Value,
    ) -> Result<Option<Self>, FieldError> {
        // TODO: check validity of descripto.label
        assert!(descriptor.name.is_some());
        assert!(!matches!(descriptor.label.unwrap(), FieldLabel::Required));
        assert!(descriptor.r#type.is_some());
        assert!(descriptor.number.is_some());

        let name = descriptor.name.clone().unwrap();
        let field_type = descriptor.r#type.unwrap();
        let number = descriptor.number.unwrap();

        // If the user is explicitly sending a null field, we considered it absent from the wire.
        if matches!(value, Value::Null) {
            return Ok(None);
        }

        let field = match field_type {
            FieldType::Double => try_new_double(&value, &name, number),
            FieldType::Float => try_new_float(&value, &name, number),
            FieldType::Int64 => try_new_int64(&value, &name, number),
            FieldType::UInt64 => try_new_uint64(&value, &name, number),
            FieldType::Int32 => try_new_int32(&value, &name, number),
            FieldType::Fixed64 => try_new_fixed64(&value, &name, number),
            FieldType::Fixed32 => try_new_fixed32(&value, &name, number),
            FieldType::Bool => try_new_bool(&value, &name, number),
            FieldType::String => try_new_string(value, &name, number),
            FieldType::Group => todo!(),
            FieldType::Message => try_new_message(descriptor, symbols, value, &name, number),
            FieldType::Bytes => try_new_bytes(&value, &name, number),
            FieldType::UInt32 => try_new_uint32(&value, &name, number),
            FieldType::Enum => todo!(),
            FieldType::SFixed32 => try_new_sfixed32(&value, &name, number),
            FieldType::SFixed64 => try_new_sfixed64(&value, &name, number),
            FieldType::SInt32 => try_new_sint32(&value, &name, number),
            FieldType::SInt64 => try_new_sint64(&value, &name, number),
        }?;

        if !descriptor.has_explicit_presence() && field.equals_default() {
            Ok(None)
        } else {
            Ok(Some(field))
        }
    }

    /// Returns `true` if this field is equal to its default value.
    ///
    /// We're using it to transmit only the fields that have non-defualt value.
    fn equals_default(&self) -> bool {
        match &self.kind {
            FieldKind::String(v) => v.is_empty(),
            FieldKind::Bool(v) => !v,
            FieldKind::Array(_) => false,
            // A nested message with zero set sub-fields is canonicalized to "unset" on the wire.
            // We may need to adapt this code if a user want to explicitely send an empty message.
            FieldKind::Message(v) => v.is_empty(),
            FieldKind::SFixed32(v) => *v == 0,
            FieldKind::Int32(v) => *v == 0,
            FieldKind::SInt32(v) => *v == 0,
            FieldKind::UInt32(v) => *v == 0,
            FieldKind::Fixed32(v) => *v == 0,
            FieldKind::SFixed64(v) => *v == 0,
            FieldKind::Int64(v) => *v == 0,
            FieldKind::SInt64(v) => *v == 0,
            FieldKind::UInt64(v) => *v == 0,
            FieldKind::Fixed64(v) => *v == 0,
            FieldKind::Double(v) => *v == 0.0,
            FieldKind::Float(v) => *v == 0.0,
            FieldKind::Bytes(v) => v.is_empty(),
        }
    }
}

/// Matches every key of `json` against `message`'s field descriptors and recurse.
pub fn parse_fields(
    message: &DescriptorProto,
    symbols: &SymbolTable,
    json: Map<String, Value>,
) -> Result<Vec<Field>, FieldError> {
    let mut fields = Vec::new();
    for (name, value) in json {
        let field_desc = message
            .fields
            .iter()
            .find(|f| f.name.as_deref() == Some(&name))
            .ok_or(FieldError::UnknownJsonField {
                field: name.clone(),
                type_name: message.fqn.clone(),
            })?;
        if let Some(field) = Field::try_new(field_desc, symbols, value)? {
            fields.push(field);
        }
    }
    Ok(fields)
}

/// Creates a new `Field` instance from a JSON `value` representing an `sfixed32`.
fn try_new_sfixed32(value: &Value, name: &str, number: u32) -> Result<Field, FieldError> {
    let v = parse_i32(value, name)?;
    Ok(Field {
        kind: FieldKind::SFixed32(v),
        number,
    })
}

/// Creates a new `Field` instance from a JSON `value` representing an `sfixed64`.
fn try_new_sfixed64(value: &Value, name: &str, number: u32) -> Result<Field, FieldError> {
    let v = parse_i64(value, name)?;
    Ok(Field {
        kind: FieldKind::SFixed64(v),
        number,
    })
}

/// Creates a new `Field` instance from a JSON `value` representing an `int64`.
fn try_new_int64(value: &Value, name: &str, number: u32) -> Result<Field, FieldError> {
    let v = parse_i64(value, name)?;
    Ok(Field {
        kind: FieldKind::Int64(v),
        number,
    })
}

/// Creates a new `Field` instance from a JSON `value` representing a `sint64`.
fn try_new_sint64(value: &Value, name: &str, number: u32) -> Result<Field, FieldError> {
    let v = parse_i64(value, name)?;
    Ok(Field {
        kind: FieldKind::SInt64(v),
        number,
    })
}

/// Creates a new `Field` instance from a JSON `value` representing a `uint64`.
fn try_new_uint64(value: &Value, name: &str, number: u32) -> Result<Field, FieldError> {
    let v = parse_u64(value, name)?;
    Ok(Field {
        kind: FieldKind::UInt64(v),
        number,
    })
}

/// Creates a new `Field` instance from a JSON `value` representing a `fixed64`.
fn try_new_fixed64(value: &Value, name: &str, number: u32) -> Result<Field, FieldError> {
    let v = parse_u64(value, name)?;
    Ok(Field {
        kind: FieldKind::Fixed64(v),
        number,
    })
}

/// Creates a new `Field` instance from a JSON `value` representing a `double`.
fn try_new_double(value: &Value, name: &str, number: u32) -> Result<Field, FieldError> {
    let v = parse_f64(value, name)?;
    Ok(Field {
        kind: FieldKind::Double(v),
        number,
    })
}

/// Creates a new `Field` instance from a JSON `value` representing a `float`.
fn try_new_float(value: &Value, name: &str, number: u32) -> Result<Field, FieldError> {
    // TODO: we don't support for the moment JSON string like "Infinity", "-Infinity", "NaN"
    let v = parse_f32(value, name)?;
    Ok(Field {
        kind: FieldKind::Float(v),
        number,
    })
}

/// Standard-alphabet base64 decoder with indifferent padding.
const BASE64_STD: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// URL-safe alphabet base64 decoder with indifferent padding.
const BASE64_URL: GeneralPurpose = GeneralPurpose::new(
    &alphabet::URL_SAFE,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// Creates a new `Field` instance from a JSON `value` representing a `bytes` field.
fn try_new_bytes(value: &Value, name: &str, number: u32) -> Result<Field, FieldError> {
    let v = parse_bytes(value, name)?;
    Ok(Field {
        kind: FieldKind::Bytes(v),
        number,
    })
}

/// Creates a new `Field` instance, named `name` and numbered `number`, from a JSON `value`
/// representing an `int32`.
fn try_new_int32(value: &Value, name: &str, number: u32) -> Result<Field, FieldError> {
    let v = parse_i32(value, name)?;
    Ok(Field {
        kind: FieldKind::Int32(v),
        number,
    })
}

/// Creates a new `Field` instance, named `name` and numbered `number`, from a JSON `value`
/// representing a `sint32`.
fn try_new_sint32(value: &Value, name: &str, number: u32) -> Result<Field, FieldError> {
    let v = parse_i32(value, name)?;
    Ok(Field {
        kind: FieldKind::SInt32(v),
        number,
    })
}

/// Creates a new `Field` instance, named `name` and numbered `number`, from a JSON `value`
/// representing a `bool`.
fn try_new_bool(value: &Value, name: &str, number: u32) -> Result<Field, FieldError> {
    let Value::Bool(v) = value else {
        return Err(FieldError::InvalidJsonInputType {
            field: name.to_string(),
            expected: "boolean".to_string(),
            actual: type_of_value(value).to_string(),
        });
    };
    Ok(Field {
        kind: FieldKind::Bool(*v),
        number,
    })
}

/// Creates a new `Field` instance, named `name` and numbered `number`, from a JSON `value`
/// representing a `fixed32`.
fn try_new_fixed32(value: &Value, name: &str, number: u32) -> Result<Field, FieldError> {
    let v = parse_u32(value, name)?;
    Ok(Field {
        kind: FieldKind::Fixed32(v),
        number,
    })
}

/// Creates a new `Field` instance, named `name` and numbered `number`, from a JSON `value`
/// representing an `uint32`.
fn try_new_uint32(value: &Value, name: &str, number: u32) -> Result<Field, FieldError> {
    let v = parse_u32(value, name)?;
    Ok(Field {
        kind: FieldKind::UInt32(v),
        number,
    })
}

/// Creates a new `Field` instance, named `name` and numbered `number`, from a JSON `value`
/// representing a message.
fn try_new_message(
    descriptor: &FieldDescriptorProto,
    symbols: &SymbolTable,
    value: Value,
    name: &str,
    number: u32,
) -> Result<Field, FieldError> {
    assert!(descriptor.type_name.is_some());
    // Do we need to distinguish between message and map ?
    let Value::Object(obj) = value else {
        return Err(FieldError::InvalidJsonInputType {
            field: name.to_string(),
            expected: "object".to_string(),
            actual: type_of_value(&value).to_string(),
        });
    };
    let type_name = descriptor.type_name.as_deref().unwrap();
    let msg_descriptor = symbols
        .find_message(type_name)
        .ok_or(FieldError::UnresolvedType {
            field: name.to_string(),
            type_name: type_name.to_string(),
        })?;
    let fields = parse_fields(msg_descriptor, symbols, obj)?;
    Ok(Field {
        kind: FieldKind::Message(fields),
        number,
    })
}

/// Creates a new `Field` instance, named `name` and numbered `number`, from a JSON `value`
/// representing a message string.
fn try_new_string(value: Value, name: &str, number: u32) -> Result<Field, FieldError> {
    match value {
        Value::String(value) => {
            let kind = FieldKind::String(value);
            Ok(Field { kind, number })
        }
        actual => {
            let expected = "string".to_string();
            let actual = type_of_value(&actual).to_string();
            let err = FieldError::InvalidJsonInputType {
                field: name.to_string(),
                expected,
                actual,
            };
            Err(err)
        }
    }
}

impl Field {
    pub fn encode(&self, writer: &mut Writer) {
        match &self.kind {
            FieldKind::String(value) => writer.write_string_field(self.number, value),
            FieldKind::Bool(v) => writer.write_bool_field(self.number, *v),
            FieldKind::Array(_) => todo!(),
            FieldKind::Message(fields) => {
                // Sort sub-fields by their number so encoding is deterministic.
                let mut sorted: Vec<&Field> = fields.iter().collect();
                sorted.sort_by_key(|f| f.number);

                // Encode the sub-message into a scratch writer, then write it as a length-delimited
                // sub-message on the outer writer. We need this because we don't knwo the size of
                // all messa's fields before encoding.
                let mut inner = Writer::new();
                for field in sorted {
                    field.encode(&mut inner);
                }
                writer.write_message_field(self.number, inner.bytes());
            }
            FieldKind::SFixed32(v) => writer.write_sfixed32_field(self.number, *v),
            FieldKind::Int32(v) => writer.write_int32_field(self.number, *v),
            FieldKind::SInt32(v) => writer.write_sint32_field(self.number, *v),
            FieldKind::UInt32(v) => writer.write_uint32_field(self.number, *v),
            FieldKind::Fixed32(v) => writer.write_fixed32_field(self.number, *v),
            FieldKind::SFixed64(v) => writer.write_sfixed64_field(self.number, *v),
            FieldKind::Int64(v) => writer.write_int64_field(self.number, *v),
            FieldKind::SInt64(v) => writer.write_sint64_field(self.number, *v),
            FieldKind::UInt64(v) => writer.write_uint64_field(self.number, *v),
            FieldKind::Fixed64(v) => writer.write_fixed64_field(self.number, *v),
            FieldKind::Double(v) => writer.write_double_field(self.number, *v),
            FieldKind::Float(v) => writer.write_float_field(self.number, *v),
            FieldKind::Bytes(v) => writer.write_bytes_field(self.number, v),
        }
    }
}

/// Extracts an `i32` from a JSON [`Value`].
fn parse_i32(value: &Value, name: &str) -> Result<i32, FieldError> {
    let n = try_integer_from(value, name)?;
    n.as_i64()
        .and_then(|v| i32::try_from(v).ok())
        .ok_or(FieldError::JsonNumberOutOfRange {
            field: name.to_string(),
            value: n.to_string(),
        })
}

/// Extracts a `u32` from a JSON [`Value`].
fn parse_u32(value: &Value, name: &str) -> Result<u32, FieldError> {
    let n = try_integer_from(value, name)?;
    n.as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or(FieldError::JsonNumberOutOfRange {
            field: name.to_string(),
            value: n.to_string(),
        })
}

/// Extracts an `i64` from a JSON [`Value`].
fn parse_i64(value: &Value, name: &str) -> Result<i64, FieldError> {
    // We accept both JSON numbers and JSON strings, proto3 JSON canonicalizes 64-bit integers as
    // strings to avoid the f64 precision loss that JSON parsers apply to numbers above 2^53.
    // See <https://protobuf.dev/programming-guides/json/>
    match value {
        Value::Number(n) => n.as_i64().ok_or(FieldError::JsonNumberOutOfRange {
            field: name.to_string(),
            value: n.to_string(),
        }),
        Value::String(s) => s
            .parse::<i64>()
            .map_err(|_| FieldError::InvalidStringAsInteger {
                field: name.to_string(),
                value: s.clone(),
            }),
        _ => Err(FieldError::InvalidJsonInputType {
            field: name.to_string(),
            expected: "integer or string".to_string(),
            actual: type_of_value(value).to_string(),
        }),
    }
}

/// Extracts a `u64` from a JSON [`Value`].
fn parse_u64(value: &Value, name: &str) -> Result<u64, FieldError> {
    // We accept both JSON numbers and JSON strings, proto3 JSON canonicalizes 64-bit integers as
    // strings to avoid the f64 precision loss that JSON parsers apply to numbers above 2^53.
    // See <https://protobuf.dev/programming-guides/json/>
    match value {
        Value::Number(n) => n.as_u64().ok_or(FieldError::JsonNumberOutOfRange {
            field: name.to_string(),
            value: n.to_string(),
        }),
        Value::String(s) => s
            .parse::<u64>()
            .map_err(|_| FieldError::InvalidStringAsInteger {
                field: name.to_string(),
                value: s.clone(),
            }),
        _ => Err(FieldError::InvalidJsonInputType {
            field: name.to_string(),
            expected: "integer or string".to_string(),
            actual: type_of_value(value).to_string(),
        }),
    }
}

/// Extracts an `f64` from a JSON [`Value`].
fn parse_f64(value: &Value, name: &str) -> Result<f64, FieldError> {
    let Value::Number(n) = value else {
        return Err(FieldError::InvalidJsonInputType {
            field: name.to_string(),
            expected: "number".to_string(),
            actual: type_of_value(value).to_string(),
        });
    };
    n.as_f64().ok_or(FieldError::JsonNumberOutOfRange {
        field: name.to_string(),
        value: n.to_string(),
    })
}

/// Extracts a byte vector from a JSON [`Value`].
fn parse_bytes(value: &Value, name: &str) -> Result<Vec<u8>, FieldError> {
    // Expects a base64 string. Accepts both the standard alphabet (`+`/`/`) and the url-safe alphabet
    // (`-`/`_`), with or without padding, see <https://protobuf.dev/programming-guides/json/>.
    //
    // > JSON value will be the data encoded as a string using standard base64 encoding with
    // > paddings. Either standard or URL-safe base64 encoding with/without paddings are accepted.
    let Value::String(s) = value else {
        return Err(FieldError::InvalidJsonInputType {
            field: name.to_string(),
            expected: "base64 string".to_string(),
            actual: type_of_value(value).to_string(),
        });
    };
    BASE64_STD
        .decode(s)
        .or_else(|_| BASE64_URL.decode(s))
        .map_err(|_| FieldError::InvalidBase64 {
            field: name.to_string(),
            value: s.clone(),
        })
}

/// Extracts an `f32` from a JSON [`Value`].
fn parse_f32(value: &Value, name: &str) -> Result<f32, FieldError> {
    // Delegates to [`parse_f64`] for the shape check, then narrows. Values outside the `f32` range
    // silently coerce to `±f32::INFINITY` — this matches Google's protoc canonical behavior.
    parse_f64(value, name).map(|f| f as f32)
}

fn type_of_value(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Returns a JSON `Value` number from a generic `value` for field named `name`.
fn try_integer_from<'value>(
    value: &'value Value,
    name: &str,
) -> Result<&'value Number, FieldError> {
    let Value::Number(n) = value else {
        return Err(FieldError::InvalidJsonInputType {
            field: name.to_string(),
            expected: "integer".to_string(),
            actual: type_of_value(value).to_string(),
        });
    };
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // parse_i32 tests

    #[test]
    fn parse_i32_accepts_positive() {
        assert_eq!(parse_i32(&json!(42), "field").unwrap(), 42);
    }

    #[test]
    fn parse_i32_accepts_bounds() {
        assert_eq!(parse_i32(&json!(i32::MIN), "field").unwrap(), i32::MIN);
        assert_eq!(parse_i32(&json!(i32::MAX), "field").unwrap(), i32::MAX);
    }

    #[test]
    fn parse_i32_rejects_out_of_range() {
        // One past i32::MAX (still fits in the i64 that serde_json uses internally).
        let too_big = parse_i32(&json!(i32::MAX as i64 + 1), "field").unwrap_err();
        assert!(matches!(too_big, FieldError::JsonNumberOutOfRange { .. }));
        // One before i32::MIN.
        let too_small = parse_i32(&json!(i32::MIN as i64 - 1), "field").unwrap_err();
        assert!(matches!(too_small, FieldError::JsonNumberOutOfRange { .. }));
    }

    #[test]
    fn parse_i32_rejects_float() {
        let err = parse_i32(&json!(1.5), "field").unwrap_err();
        assert!(matches!(err, FieldError::JsonNumberOutOfRange { .. }));
    }

    #[test]
    fn parse_i32_rejects_non_number() {
        let err = parse_i32(&json!("42"), "field").unwrap_err();
        assert!(matches!(err, FieldError::InvalidJsonInputType { .. }));
    }

    // parse_u32 tests

    #[test]
    fn parse_u32_accepts_integer() {
        assert_eq!(parse_u32(&json!(42), "field").unwrap(), 42);
    }

    #[test]
    fn parse_u32_accepts_u32_max() {
        assert_eq!(parse_u32(&json!(u32::MAX), "field").unwrap(), u32::MAX);
    }

    #[test]
    fn parse_u32_rejects_negative() {
        let err = parse_u32(&json!(-1), "field").unwrap_err();
        assert!(matches!(err, FieldError::JsonNumberOutOfRange { .. }));
    }

    #[test]
    fn parse_u32_rejects_too_big() {
        // One past u32::MAX (fits in u64 for serde_json).
        let err = parse_u32(&json!(u32::MAX as u64 + 1), "field").unwrap_err();
        assert!(matches!(err, FieldError::JsonNumberOutOfRange { .. }));
    }

    #[test]
    fn parse_u32_rejects_non_number() {
        let err = parse_u32(&json!("42"), "field").unwrap_err();
        assert!(matches!(err, FieldError::InvalidJsonInputType { .. }));
    }

    // parse_i64 tests

    #[test]
    fn parse_i64_accepts_integer() {
        assert_eq!(parse_i64(&json!(42), "field").unwrap(), 42);
    }

    #[test]
    fn parse_i64_accepts_negative() {
        assert_eq!(parse_i64(&json!(-1), "field").unwrap(), -1);
    }

    #[test]
    fn parse_i64_accepts_bounds() {
        assert_eq!(parse_i64(&json!(i64::MIN), "field").unwrap(), i64::MIN);
        assert_eq!(parse_i64(&json!(i64::MAX), "field").unwrap(), i64::MAX);
    }

    #[test]
    fn parse_i64_rejects_float() {
        let err = parse_i64(&json!(1.5), "field").unwrap_err();
        assert!(matches!(err, FieldError::JsonNumberOutOfRange { .. }));
    }

    #[test]
    fn parse_i64_accepts_string() {
        // Proto3 JSON canonical form for 64-bit integers.
        assert_eq!(parse_i64(&json!("-264836"), "field").unwrap(), -264836);
    }

    #[test]
    fn parse_i64_rejects_unparseable_string() {
        let err = parse_i64(&json!("not-a-number"), "field").unwrap_err();
        assert!(matches!(err, FieldError::InvalidStringAsInteger { .. }));
    }

    #[test]
    fn parse_i64_rejects_bool() {
        // Neither a number nor a string — the "wrong type" case.
        let err = parse_i64(&json!(true), "field").unwrap_err();
        assert!(matches!(err, FieldError::InvalidJsonInputType { .. }));
    }

    // parse_u64 tests

    #[test]
    fn parse_u64_accepts_integer() {
        assert_eq!(parse_u64(&json!(42), "field").unwrap(), 42);
    }

    #[test]
    fn parse_u64_accepts_u64_max() {
        assert_eq!(parse_u64(&json!(u64::MAX), "field").unwrap(), u64::MAX);
    }

    #[test]
    fn parse_u64_rejects_negative() {
        // as_u64() returns None for negative values.
        let err = parse_u64(&json!(-1), "field").unwrap_err();
        assert!(matches!(err, FieldError::JsonNumberOutOfRange { .. }));
    }

    #[test]
    fn parse_u64_rejects_float() {
        // as_u64() returns None for non-integer JSON numbers.
        let err = parse_u64(&json!(1.5), "field").unwrap_err();
        assert!(matches!(err, FieldError::JsonNumberOutOfRange { .. }));
    }

    #[test]
    fn parse_u64_accepts_string() {
        // Proto3 JSON canonical form for 64-bit integers.
        assert_eq!(parse_u64(&json!("617"), "field").unwrap(), 617);
    }

    #[test]
    fn parse_u64_rejects_negative_string() {
        // "-1".parse::<u64>() fails, routes to InvalidStringAsInteger.
        let err = parse_u64(&json!("-1"), "field").unwrap_err();
        assert!(matches!(err, FieldError::InvalidStringAsInteger { .. }));
    }

    #[test]
    fn parse_u64_rejects_bool() {
        let err = parse_u64(&json!(true), "field").unwrap_err();
        assert!(matches!(err, FieldError::InvalidJsonInputType { .. }));
    }

    // parse_f64 tests

    #[test]
    fn parse_f64_accepts_float() {
        assert_eq!(parse_f64(&json!(1.5), "field").unwrap(), 1.5);
    }

    #[test]
    fn parse_f64_accepts_integer() {
        // JSON integers coerce to f64.
        assert_eq!(parse_f64(&json!(42), "field").unwrap(), 42.0);
    }

    #[test]
    fn parse_f64_accepts_negative() {
        assert_eq!(parse_f64(&json!(-1.5), "field").unwrap(), -1.5);
    }

    #[test]
    fn parse_f64_rejects_non_number() {
        // Uses a different shape check than the integer parsers — "expected: number".
        let err = parse_f64(&json!("1.5"), "field").unwrap_err();
        assert!(matches!(err, FieldError::InvalidJsonInputType { .. }));
    }

    // parse_f32 tests

    #[test]
    fn parse_f32_accepts_float() {
        assert_eq!(parse_f32(&json!(1.5), "field").unwrap(), 1.5);
    }

    #[test]
    fn parse_f32_accepts_integer() {
        assert_eq!(parse_f32(&json!(42), "field").unwrap(), 42.0);
    }

    #[test]
    fn parse_f32_accepts_negative() {
        assert_eq!(parse_f32(&json!(-1.5), "field").unwrap(), -1.5);
    }

    #[test]
    fn parse_f32_narrows_silently_on_overflow() {
        // f64::MAX doesn't fit in f32 — coerces to +infinity rather than erroring.
        let v = parse_f32(&json!(f64::MAX), "field").unwrap();
        assert!(v.is_infinite() && v.is_sign_positive());
    }

    #[test]
    fn parse_f32_rejects_non_number() {
        let err = parse_f32(&json!("1.5"), "field").unwrap_err();
        assert!(matches!(err, FieldError::InvalidJsonInputType { .. }));
    }

    // parse_bytes tests

    #[test]
    fn parse_bytes_accepts_standard_base64_with_padding() {
        // "hello" => base64 standard alphabet, padded.
        assert_eq!(parse_bytes(&json!("aGVsbG8="), "field").unwrap(), b"hello");
    }

    #[test]
    fn parse_bytes_accepts_standard_base64_without_padding() {
        // Same "hello" without the trailing `=`.
        assert_eq!(parse_bytes(&json!("aGVsbG8"), "field").unwrap(), b"hello");
    }

    #[test]
    fn parse_bytes_accepts_url_safe_alphabet() {
        // The bytes `[0xfb, 0xff, 0xff]` encode to `+///` in standard base64 and `-___` in
        // url-safe. The `-` character is not in the standard alphabet, so this input can only
        // decode via the url-safe fallback engine.
        assert_eq!(
            parse_bytes(&json!("-___"), "field").unwrap(),
            vec![0xfb, 0xff, 0xff]
        );
    }

    #[test]
    fn parse_bytes_accepts_empty() {
        assert_eq!(parse_bytes(&json!(""), "field").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn parse_bytes_rejects_invalid_base64() {
        // `!` is not in either base64 alphabet.
        let err = parse_bytes(&json!("not!base64"), "field").unwrap_err();
        assert!(matches!(err, FieldError::InvalidBase64 { .. }));
    }

    #[test]
    fn parse_bytes_rejects_non_string() {
        let err = parse_bytes(&json!(42), "field").unwrap_err();
        assert!(matches!(err, FieldError::InvalidJsonInputType { .. }));
    }
}
