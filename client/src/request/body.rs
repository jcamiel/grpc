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

use super::encoder::Field;
use crate::schema::descriptor::DescriptorProto;

#[derive(Debug)]
pub struct RequestBody {
    fields: Vec<Field>,
}

#[derive(Debug)]
pub enum RequestBodyError {
    InvalidJson {
        error: String,
    },
    NotJsonObject,
    UnknownField {
        input_message: String,
        field: String,
    },
    InvalidField {
        error: String,
    },
}

impl fmt::Display for RequestBodyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RequestBodyError::InvalidJson { error } => {
                write!(f, "invalid request body, {error}")
            }
            RequestBodyError::NotJsonObject => write!(f, "expecting JSON Object"),
            RequestBodyError::UnknownField {
                input_message,
                field,
            } => write!(
                f,
                "invalid request body, message type '{input_message}' has no known field named '{field}'"
            ),
            RequestBodyError::InvalidField { error } => write!(f, "invalid request body, {error}"),
        }
    }
}

impl RequestBody {
    pub fn encode(&self) -> &[u8] {
        for field in self.fields.iter() {
            println!("{field:#?}");
        }
        &[]
    }

    pub fn try_new(
        bytes: &[u8],
        input_message: &DescriptorProto,
    ) -> Result<RequestBody, RequestBodyError> {
        let bytes = bytes.trim_ascii();
        let mut fields = Vec::new();
        let json_body = if !bytes.is_empty() {
            let v: Value =
                serde_json::from_slice(bytes).map_err(|e| RequestBodyError::InvalidJson {
                    error: e.to_string(),
                })?;
            match v {
                Value::Null
                | Value::Bool(_)
                | Value::Number(_)
                | Value::String(_)
                | Value::Array(_) => return Err(RequestBodyError::NotJsonObject),
                Value::Object(map) => map,
            }
        } else {
            Map::new()
        };

        // Iterate on each field
        for (name, value) in json_body.into_iter() {
            let field_desc = input_message
                .fields
                .iter()
                .find(|f| f.name.as_deref() == Some(&name))
                .ok_or_else(|| RequestBodyError::UnknownField {
                    input_message: input_message.fqn.clone(),
                    field: name.clone(),
                })?;

            let field =
                Field::try_new(field_desc, value).map_err(|e| RequestBodyError::InvalidField {
                    error: e.to_string(),
                })?;
            fields.push(field);
        }

        Ok(RequestBody { fields })
    }
}
