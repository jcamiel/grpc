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

//! Protobuf wire-format primitives, independent of any schema.

use std::fmt;
use std::fmt::Formatter;
use crate::wire::reader::ReaderError;

pub mod reader;
pub mod writer;

/// A byte position in a bytes stream (0-based index).
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct BytePos(usize);

/// Represents a wire type (the type part of a record value)
/// From <https://protobuf.dev/programming-guides/encoding>
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum WireType {
    /// int32, int64, uint32, uint64, sint32, sint64, bool, enum
    VarInt = 0,
    /// fixed64, sfixed64, double
    I64 = 1,
    /// string, bytes, embedded messages, packed repeated fields
    Len = 2,
    /// group start (deprecated)
    SGroup = 3,
    /// group end (deprecated)
    EGroup = 4,
    /// fixed32, sfixed32, float
    I32 = 5,
}

impl TryFrom<u8> for WireType {
    type Error = ReaderError;

    fn try_from(value: u8) -> Result<Self, ReaderError> {
        match value {
            0 => Ok(WireType::VarInt),
            1 => Ok(WireType::I64),
            2 => Ok(WireType::Len),
            3 => Ok(WireType::SGroup),
            4 => Ok(WireType::EGroup),
            5 => Ok(WireType::I32),
            _ => Err(ReaderError::InvalidWireType { wire_type: value }),
        }
    }
}

impl fmt::Display for WireType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            WireType::VarInt => write!(f, "VARINT"),
            WireType::I64 => write!(f, "I64"),
            WireType::Len => write!(f, "LEN"),
            WireType::SGroup => write!(f, "SGROUP"),
            WireType::EGroup => write!(f, "EGROUP"),
            WireType::I32 => write!(f, "I32"),
        }
    }
}
