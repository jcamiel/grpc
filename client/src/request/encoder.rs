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
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;

use serde_json::Value;

use super::Request;
use super::body::RequestBody;
use crate::schema::descriptor::{FieldDescriptorProto, FieldLabel, FieldType};
use crate::schema::symbols::SymbolTable;
use crate::wire::writer::Writer;

impl Request<'_> {
    pub fn encode(&self, writer: &mut Writer) {
        self.body().encode(writer);
    }
}

impl RequestBody {
    pub fn encode(&self, writer: &mut Writer) {
        // Sort fields with number before encoding so our fields are always encoded in the
        // same order (based on the descriptor).
        let mut fields: Vec<&Field> = self.fields().iter().collect();
        fields.sort_by_key(|f| f.number);

        for field in fields.iter() {
            field.encode(writer);
        }
    }
}

#[derive(Debug)]
pub struct Field {
    kind: FieldKind,
    number: u32,
}

#[derive(Debug)]
pub enum FieldKind {
    String(String),
    Bool(bool),
    Array(Vec<Field>),
    Message(HashMap<String, Field>),
    SFixed32(i32),
}

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
            FieldType::Int32 => todo!(),
            FieldType::Fixed64 => todo!(),
            FieldType::Fixed32 => todo!(),
            FieldType::Bool => todo!(),
            FieldType::String => Self::try_new_string(value, &name, number),
            FieldType::Group => todo!(),
            FieldType::Message => Self::try_new_message(descriptor, symbols, value, &name, number),
            FieldType::Bytes => todo!(),
            FieldType::UInt32 => todo!(),
            FieldType::Enum => todo!(),
            FieldType::SFixed32 => Self::try_new_sfixed32(&value, name, number),
            FieldType::SFixed64 => todo!(),
            FieldType::SInt32 => todo!(),
            FieldType::SInt64 => todo!(),
        }?;
        Ok(Some(field))
    }

    fn try_new_sfixed32(value: &Value, name: String, number: u32) -> Result<Field, FieldError> {
        let Value::Number(n) = value else {
            return Err(FieldError::InvalidJsonInputType {
                field: name,
                expected: "integer".to_string(),
                actual: type_of_value(&value).to_string(),
            });
        };
        let Some(v) = n.as_i64().and_then(|v| i32::try_from(v).ok()) else {
            return Err(FieldError::JsonNumberOutOfRange {
                field: name,
                value: n.to_string(),
            });
        };
        Ok(Field {
            kind: FieldKind::SFixed32(v),
            number,
        })
    }

    fn try_new_message(
        descriptor: &FieldDescriptorProto,
        symbols: &SymbolTable,
        value: Value,
        name: &str,
        number: u32,
    ) -> Result<Field, FieldError> {
        assert!(descriptor.type_name.is_some());
        match value {
            // Do we need to distinguish between message and map ?
            Value::Object(value) => {
                let type_name = descriptor.type_name.as_deref().unwrap();
                let msg_descriptor =
                    symbols
                        .find_message(type_name)
                        .ok_or(FieldError::UnresolvedType {
                            field: name.to_string(),
                            type_name: type_name.to_string(),
                        })?;
                let mut map = HashMap::new();
                for (field_name, field_value) in value.into_iter() {
                    let field_desc = msg_descriptor
                        .fields
                        .iter()
                        .find(|f| f.name.as_deref() == Some(&field_name))
                        .ok_or(FieldError::UnknownJsonField {
                            field: field_name.clone(),
                            type_name: msg_descriptor.fqn.to_string(),
                        })?;
                    let field = Field::try_new(field_desc, symbols, field_value)?;
                    if let Some(field) = field {
                        map.insert(field_name, field);
                    }
                }
                let kind = FieldKind::Message(map);
                let field = Field { kind, number };
                Ok(field)
            }
            actual => {
                let expected = "object".to_string();
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
}

impl Field {
    pub fn encode(&self, writer: &mut Writer) {
        match &self.kind {
            FieldKind::String(value) => {
                writer.write_string_field(self.number, &value);
            }
            FieldKind::Bool(_) => todo!(),
            FieldKind::Array(_) => todo!(),
            FieldKind::Message(map) => {
                // Sort sub-fields by their number so encoding is deterministic.
                let mut fields: Vec<&Field> = map.values().collect();
                fields.sort_by_key(|f| f.number);

                // Encode the sub-message into a scratch writer, then write it as a length-delimited
                // sub-message on the outer writer.
                let mut inner = Writer::new();
                for field in fields {
                    field.encode(&mut inner);
                }
                writer.write_message_field(self.number, inner.bytes());
            }
            FieldKind::SFixed32(v) => {
                writer.write_sfixed32_field(self.number, *v);
            }
        }
    }
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
