use super::reader::{Reader, ReaderError, WireType};
use std::fmt;
use std::fmt::Formatter;

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
