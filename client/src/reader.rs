use std::fmt;
use std::fmt::Formatter;

/// Errors returned by [`decode`].
#[derive(Debug)]
pub enum ReaderError {
    /// Unexpected end of file.
    Eof,
    InvalidFieldNumber,
    /// Error in parsing field `name` for this `entity`.
    InvalidField {
        field: String,
        entity: String,
        expected_wire_type: WireType,
        actual_wire_type: WireType,
    },
    InvalidInt32,
    InvalidUtf8Bytes,
    InvalidWireType {
        wire_type: u8,
    },
    /// Data is valid protobug bytes but not supported for the moment.
    LegacyWireType {
        wire_type: WireType,
    },
    UnsupportedSyntax {
        syntax: String,
    },
    Generic,
}

impl fmt::Display for ReaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ReaderError::Eof => write!(f, "ReaderError::Eof"),
            ReaderError::InvalidFieldNumber => write!(f, "ReaderError::InvalidFieldNumber"),
            ReaderError::InvalidField {
                field,
                entity,
                expected_wire_type,
                actual_wire_type,
            } => write!(
                f,
                "Invalid field {entity}:{field} expected {expected_wire_type}, actual {actual_wire_type}"
            ),
            ReaderError::InvalidInt32 => write!(f, "ReaderError::InvalidInt32"),
            ReaderError::InvalidUtf8Bytes => write!(f, "ReaderError::InvalidUtf8Bytes"),
            ReaderError::InvalidWireType { .. } => write!(f, "ReaderError::InvalidWireType"),
            ReaderError::LegacyWireType { .. } => write!(f, "ReaderError::LegacyWireType"),
            ReaderError::UnsupportedSyntax { .. } => write!(f, "ReaderError::UnsupportedSyntax"),
            ReaderError::Generic => write!(f, "ReaderError::Generic"),
        }
    }
}

pub struct Reader<'input> {
    input: &'input [u8],
    pos: BytePos,
}

/// Represents a wire type (the type part of a record value)
/// From <https://protobuf.dev/programming-guides/encoding>
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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
    pub fn read_varint(&mut self) -> Result<u64, ReaderError> {
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

    /// Reads an int32 protobuf (unsigned).
    pub fn read_int32(&mut self) -> Result<u32, ReaderError> {
        let value = self.read_varint()?;
        u32::try_from(value).map_err(|_| ReaderError::InvalidInt32)
    }

    /// Reads a key.
    pub fn read_key(&mut self) -> Result<(u32, WireType), ReaderError> {
        let key = self.read_varint()?;
        let field_number = key >> 3;
        if field_number == 0 || field_number > 0x1FFF_FFFF {
            return Err(ReaderError::InvalidFieldNumber);
        }
        let field_number = field_number as u32;
        let wire_type = WireType::try_from((key & 0x7) as u8)?;
        Ok((field_number, wire_type))
    }

    pub fn read_len_delimited(&mut self) -> Result<&[u8], ReaderError> {
        let len = self.read_varint()? as usize;
        Ok(self.read_bytes(len))
    }

    pub fn eof(&self) -> bool {
        self.pos.0 >= self.input.len()
    }

    pub fn skip(&mut self, wire_type: WireType) -> Result<(), ReaderError> {
        match wire_type {
            WireType::VarInt => {
                self.read_varint()?;
            }
            WireType::I64 => {
                self.pos.0 += 8;
            }
            WireType::Len => {
                let len = self.read_varint()? as usize;
                self.pos.0 += len;
            }
            WireType::I32 => self.pos.0 += 4,
            _ => return Err(ReaderError::LegacyWireType { wire_type }),
        }
        Ok(())
    }

    pub fn read_string(&mut self) -> Result<String, ReaderError> {
        let bytes = self.read_len_delimited()?;
        String::from_utf8(bytes.to_vec()).map_err(|_| ReaderError::InvalidUtf8Bytes)
    }
}
