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

use super::reader::{Reader, ReaderError, WireType};

#[derive(Debug)]
pub enum ParserError {
    Reader(ReaderError),
    WireTypeMismatch {
        expected: WireType,
        actual: WireType,
        field: &'static str,
        entity: &'static str,
    },
    UnsupportedSyntax {
        syntax: String,
    },
    Schema {
        cause: String,
    },
}

impl From<ReaderError> for ParserError {
    fn from(value: ReaderError) -> Self {
        ParserError::Reader(value)
    }
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ParserError::Reader(..) => write!(f, "ParserError::Reader"),
            ParserError::WireTypeMismatch { .. } => write!(f, "ParserError::WireTypeMismatch"),
            ParserError::UnsupportedSyntax { .. } => write!(f, "ParserError::UnsupportedSyntax"),
            ParserError::Schema { .. } => write!(f, "ParserError::Schema"),
        }
    }
}

pub fn string(
    field: &'static str,
    entity: &'static str,
    reader: &mut Reader,
    wt: WireType,
) -> Result<String, ParserError> {
    if wt != WireType::Len {
        return Err(ParserError::WireTypeMismatch {
            expected: WireType::Len,
            actual: wt,
            field,
            entity,
        });
    }
    let value = reader.read_string()?;
    Ok(value)
}

pub fn bool(
    field: &'static str,
    entity: &'static str,
    reader: &mut Reader,
    wt: WireType,
) -> Result<bool, ParserError> {
    if wt != WireType::VarInt {
        return Err(ParserError::WireTypeMismatch {
            expected: WireType::VarInt,
            actual: wt,
            field,
            entity,
        });
    }
    let value = reader.read_bool()?;
    Ok(value)
}

pub fn uint32(
    field: &'static str,
    entity: &'static str,
    reader: &mut Reader,
    wt: WireType,
) -> Result<u32, ParserError> {
    if wt != WireType::VarInt {
        return Err(ParserError::WireTypeMismatch {
            expected: WireType::VarInt,
            actual: wt,
            field,
            entity,
        });
    }
    let value = reader.read_uint32()?;
    Ok(value)
}

pub fn message<'input>(
    field: &'static str,
    entity: &'static str,
    reader: &'input mut Reader,
    wt: WireType,
) -> Result<&'input [u8], ParserError> {
    if wt != WireType::Len {
        return Err(ParserError::WireTypeMismatch {
            expected: WireType::Len,
            actual: wt,
            field,
            entity,
        });
    }
    let value = reader.read_len_delimited()?;
    Ok(value)
}

pub fn r#enum(
    field: &'static str,
    entity: &'static str,
    reader: &mut Reader,
    wt: WireType,
) -> Result<u64, ParserError> {
    if wt != WireType::VarInt {
        return Err(ParserError::WireTypeMismatch {
            expected: WireType::VarInt,
            actual: wt,
            field,
            entity,
        });
    }
    let value = reader.read_varint()?;
    Ok(value)
}
