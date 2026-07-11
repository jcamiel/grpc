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

use crate::request::Request;
use crate::request::body::RequestBody;
use crate::schema::descriptor::{FieldDescriptorProto, FieldLabel, FieldType};
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

        for field in self.fields().iter() {
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
}

pub enum FieldError {
    InvalidInputType {
        name: String,
        expected: String,
        actual: String,
    },
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            FieldError::InvalidInputType {
                name,
                expected,
                actual,
            } => write!(
                f,
                "bad input for field '{name}' expecting '{expected}' actual '{actual}'"
            ),
        }
    }
}

impl Field {
    pub fn try_new(descriptor: &FieldDescriptorProto, value: Value) -> Result<Self, FieldError> {
        // TODO: check validity of descripto.label
        assert!(descriptor.name.is_some());
        assert!(!matches!(descriptor.label.unwrap(), FieldLabel::Required));
        assert!(descriptor.r#type.is_some());
        assert!(descriptor.number.is_some());

        let name = descriptor.name.clone().unwrap();
        let field_type = descriptor.r#type.unwrap();
        let number = descriptor.number.unwrap();

        match field_type {
            FieldType::Double => todo!(),
            FieldType::Float => todo!(),
            FieldType::Int64 => todo!(),
            FieldType::UInt64 => todo!(),
            FieldType::Int32 => todo!(),
            FieldType::Fixed64 => todo!(),
            FieldType::Fixed32 => todo!(),
            FieldType::Bool => todo!(),
            FieldType::String => match value {
                Value::String(value) => {
                    let kind = FieldKind::String(value);
                    Ok(Field { kind, number })
                }
                actual => {
                    let expected = "string".to_string();
                    let actual = type_of_value(&actual).to_string();
                    let err = FieldError::InvalidInputType {
                        expected,
                        actual,
                        name,
                    };
                    Err(err)
                }
            },
            FieldType::Group => todo!(),
            FieldType::Message => todo!(),
            FieldType::Bytes => todo!(),
            FieldType::UInt32 => todo!(),
            FieldType::Enum => todo!(),
            FieldType::SFixed32 => todo!(),
            FieldType::SFixed64 => todo!(),
            FieldType::SInt32 => todo!(),
            FieldType::SInt64 => todo!(),
        }
    }

    pub fn encode(&self, writer: &mut Writer) {
        match &self.kind {
            FieldKind::String(value) => {
                writer.write_string_field(self.number, &value);
            }
            FieldKind::Bool(_) => todo!(),
            FieldKind::Array(_) => todo!(),
            FieldKind::Message(_) => todo!(),
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
