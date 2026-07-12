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
use serde_json::{Map, Value};
use std::fmt;
use std::fmt::Formatter;

use super::field::{Field, parse_fields};
use crate::schema::descriptor::DescriptorProto;
use crate::schema::symbols::SymbolTable;
use crate::wire::writer::Writer;

#[derive(Debug)]
pub struct RequestBody {
    fields: Vec<Field>,
}

#[derive(Debug)]
pub enum RequestBodyError {
    InvalidJson { error: String },
    NotJsonObject,
    InvalidField { error: String },
}

impl fmt::Display for RequestBodyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RequestBodyError::InvalidJson { error } => {
                write!(f, "invalid JSON request body, {error}")
            }
            RequestBodyError::NotJsonObject => {
                write!(f, "invalid request body, expecting JSON Object")
            }
            RequestBodyError::InvalidField { error } => write!(f, "invalid request body, {error}"),
        }
    }
}

impl RequestBody {
    pub fn try_new(
        bytes: &[u8],
        input_message: &DescriptorProto,
        symbols: &SymbolTable,
    ) -> Result<RequestBody, RequestBodyError> {
        let bytes = bytes.trim_ascii();
        let json_body = if bytes.is_empty() {
            Map::new()
        } else {
            let value: Value =
                serde_json::from_slice(bytes).map_err(|e| RequestBodyError::InvalidJson {
                    error: e.to_string(),
                })?;
            let Value::Object(obj) = value else {
                return Err(RequestBodyError::NotJsonObject);
            };
            obj
        };
        let fields = parse_fields(input_message, symbols, json_body).map_err(|e| {
            RequestBodyError::InvalidField {
                error: e.to_string(),
            }
        })?;
        Ok(RequestBody { fields })
    }

    pub fn encode(&self, writer: &mut Writer) {
        // Sort fields with number before encoding so our fields are always encoded in the
        // same order (based on the descriptor).
        let mut fields: Vec<&Field> = self.fields.iter().collect();
        fields.sort_by_key(|f| f.number());

        for field in fields.iter() {
            field.encode(writer);
        }
    }
}
