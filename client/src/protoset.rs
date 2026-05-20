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

use std::error;
use std::fmt;

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
#[allow(dead_code)] // variants come online with the decoder
pub enum DecodeError {
    // TODO: specific variants (truncated, invalid varint, unknown wire type, ...)
    NotImplemented,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::NotImplemented => write!(f, "protoset decoder not implemented yet"),
        }
    }
}

impl error::Error for DecodeError {}

/// Decode a serialized `FileDescriptorSet` from raw bytes.
///
/// The wire format is described at
/// <https://protobuf.dev/programming-guides/encoding>.
pub fn decode(_bytes: &[u8]) -> Result<FileDescriptorSet, DecodeError> {
    todo!("implement protobuf wire-format decode for FileDescriptorSet")
}
