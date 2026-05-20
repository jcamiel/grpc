//! Decoder for a serialized `FileDescriptorSet` — the binary artifact produced
//! by `protoc --descriptor_set_out=...`, conventionally named `*.protoset`.
//!
//! A `.protoset` file is itself a protobuf message, defined by Google's
//! `descriptor.proto`. Decoding it is therefore "just" protobuf wire-format
//! decoding (varint, length-delimited, fixed32/64, tag = field<<3 | wire-type)
//! — see the encoding spec:
//!
//! <https://protobuf.dev/programming-guides/encoding>
//!
//! Nothing is implemented yet; this module is the entry point we'll fill in.
#![allow(dead_code)]

use std::fmt;
use std::fmt::Formatter;

/// A decoded `google.protobuf.FileDescriptorSet`.
///
/// Fields will be added as the decoder grows. For now this is an opaque
/// placeholder so callers can already reason about the eventual contract.
#[derive(Debug, Default)]
pub struct FileDescriptorSet {
    // TODO: file: Vec<FileDescriptorProto>
}

/// Errors returned by [`decode`].
#[derive(Debug)]
pub enum ReaderError {
    /// Unexpected end of file.
    Eof,
    /// Invalid field number
    InvalidFieldNumber,
    /// Invalid key
    InvalidKey,
}

impl fmt::Display for ReaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ReaderError::Eof => write!(f, "ReaderError::Eof"),
            ReaderError::InvalidFieldNumber => write!(f, "ReaderError::InvalidFieldNumber"),
            ReaderError::InvalidKey => write!(f, "ReaderError::InvalidKey"),
        }
    }
}

/// From <https://protobuf.dev/programming-guides/encoding>
///
/// int32, int64, uint32, uint64, sint32, sint64, bool, enum
const WIRE_TYPE_VARINT: u8 = 0;
/// fixed64, sfixed64, double
const WIRE_TYPE_I64: u8 = 1;
/// string, bytes, embedded messages, packed repeated fields
const WIRE_TYPE_LEN: u8 = 2;
/// group start (deprecated)
const WIRE_TYPE_SGROUP: u8 = 3;
/// group end (deprecated)
const WIRE_TYPE_EGROUP: u8 = 4;
/// fixed32, sfixed32, float
const WIRE_TYPE_I32: u8 = 5;

/// Decode a serialized `FileDescriptorSet` from raw bytes.
///
/// The wire format is described at
/// <https://protobuf.dev/programming-guides/encoding>.
pub fn decode(bytes: &[u8]) -> Result<FileDescriptorSet, ReaderError> {
    let mut decoder = Reader::new(bytes);

    while decoder.pos.0 < bytes.len() {
        let (field_number, wire_type) = decoder.read_key()?;
        if field_number == 1 && wire_type == WIRE_TYPE_LEN {
            let _msg_bytes = decoder.read_len_delimited()?;
        } else {
            return Err(ReaderError::InvalidKey);
        }
    }
    Ok(FileDescriptorSet {})
}

pub struct Reader<'input> {
    input: &'input [u8],
    pos: BytePos,
}

/// A byte position in a bytes stream (0-based index).
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct BytePos(usize);

impl<'input> Reader<'input> {
    /// Creates a new decoder, with `input`.
    pub fn new(input: &'input [u8]) -> Self {
        Reader {
            input,
            pos: BytePos(0),
        }
    }

    /// Reads the next byte and advances the read position.
    fn read_byte(&mut self) -> Option<u8> {
        let b = self.input.get(self.pos.0).copied()?;
        self.pos.0 += 1;
        Some(b)
    }

    /// Reads the next n bytes and advances the read position.
    fn read_bytes(&mut self, n: usize) -> &[u8] {
        let start = self.pos.0;
        self.pos.0 += n;
        &self.input[start..start + n]
    }

    /// Reads a varint
    fn read_varint(&mut self) -> Result<u64, ReaderError> {
        let mut shift = 0;
        let mut result = 0;

        loop {
            let byte = match self.read_byte() {
                Some(b) => b,
                None => return Err(ReaderError::Eof),
            };
            result |= ((byte & 0x7F) as u64) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }

        Ok(result)
    }

    /// Reads a key.
    fn read_key(&mut self) -> Result<(u32, u8), ReaderError> {
        let key = self.read_varint()?;
        let field_number = key >> 3;
        if field_number == 0 || field_number > 0x1FFF_FFFF {
            return Err(ReaderError::InvalidFieldNumber);
        }
        let field_number = field_number as u32;
        let wire_type = (key & 0x7) as u8;
        Ok((field_number, wire_type))
    }

    fn read_len_delimited(&mut self) -> Result<&[u8], ReaderError> {
        let len = self.read_varint()? as usize;
        Ok(self.read_bytes(len))
    }
}
