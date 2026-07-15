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

use serde_json::{Map, Value};

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
                "bad input for field '{field}' expecting JSON {expected} actual {actual}"
            ),
            FieldError::UnresolvedType { field, type_name } => write!(
                f,
                "bad input for field '{field}' type '{type_name}' is unknown"
            ),
            FieldError::UnknownJsonField { field, type_name } => write!(
                f,
                "message type '{type_name}' has no known field named '{field}'"
            ),
            FieldError::JsonNumberOutOfRange { field, value } => write!(
                f,
                "bad input for field '{field}', JSON number '{value}' is out of range"
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
            FieldType::Double => todo!(),
            FieldType::Float => todo!(),
            FieldType::Int64 => todo!(),
            FieldType::UInt64 => todo!(),
            FieldType::Int32 => try_new_int32(&value, &name, number),
            FieldType::Fixed64 => todo!(),
            FieldType::Fixed32 => try_new_fixed32(&value, &name, number),
            FieldType::Bool => try_new_bool(&value, &name, number),
            FieldType::String => try_new_string(value, &name, number),
            FieldType::Group => todo!(),
            FieldType::Message => try_new_message(descriptor, symbols, value, &name, number),
            FieldType::Bytes => todo!(),
            FieldType::UInt32 => try_new_uint32(&value, &name, number),
            FieldType::Enum => todo!(),
            FieldType::SFixed32 => try_new_sfixed32(&value, &name, number),
            FieldType::SFixed64 => todo!(),
            FieldType::SInt32 => try_new_sint32(&value, &name, number),
            FieldType::SInt64 => todo!(),
        }?;
        Ok(Some(field))
    }
}

/// Creates a new `Field` instance from a JSON `value` representing an `sfixed32`.
fn try_new_sfixed32(value: &Value, name: &str, number: u32) -> Result<Field, FieldError> {
    let v = parse_i32(value, name)?;
    Ok(Field {
        kind: FieldKind::SFixed32(v),
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
            FieldKind::String(value) => writer.write_string_field(self.number, &value),
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
        }
    }
}

/// Extracts an `i32` from a JSON [`Value`].
fn parse_i32(value: &Value, name: &str) -> Result<i32, FieldError> {
    let Value::Number(n) = value else {
        return Err(FieldError::InvalidJsonInputType {
            field: name.to_string(),
            expected: "integer".to_string(),
            actual: type_of_value(value).to_string(),
        });
    };
    n.as_i64()
        .and_then(|v| i32::try_from(v).ok())
        .ok_or(FieldError::JsonNumberOutOfRange {
            field: name.to_string(),
            value: n.to_string(),
        })
}

/// Extracts a `u32` from a JSON [`Value`].
fn parse_u32(value: &Value, name: &str) -> Result<u32, FieldError> {
    let Value::Number(n) = value else {
        return Err(FieldError::InvalidJsonInputType {
            field: name.to_string(),
            expected: "integer".to_string(),
            actual: type_of_value(value).to_string(),
        });
    };
    n.as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or(FieldError::JsonNumberOutOfRange {
            field: name.to_string(),
            value: n.to_string(),
        })
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
