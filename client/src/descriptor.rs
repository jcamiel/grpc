use super::reader::{Reader, ReaderError, WireType};
use std::fmt;
use std::fmt::Formatter;

/// A decoded `google.protobuf.FileDescriptorSet`.
/// See <https://github.com/protocolbuffers/protobuf/blob/main/src/google/protobuf/descriptor.proto>
#[derive(Debug, Default)]
pub struct FileDescriptorSet {
    files: Vec<FileDescriptorProto>,
}

impl FileDescriptorSet {
    /// Decodes a serialized `FileDescriptorSet` from raw bytes.
    pub fn parse(bytes: &[u8]) -> Result<FileDescriptorSet, ReaderError> {
        let mut reader = Reader::new(bytes);
        let entity = "FileDescriptorSet";
        let mut files = Vec::new();

        while !reader.eof() {
            let (field_number, wire_type) = reader.read_tag()?;
            match field_number {
                1 => {
                    check_wire_type(entity, "file", WireType::Len, wire_type)?;
                    let bytes = reader.read_len_delimited()?;
                    let file = FileDescriptorProto::parse(bytes)?;
                    files.push(file);
                }
                _ => reader.skip(wire_type)?,
            }
        }
        Ok(FileDescriptorSet { files })
    }
}

/// See <https://github.com/protocolbuffers/protobuf/blob/v32.0/src/google/protobuf/descriptor.proto#L104>
#[derive(Debug, Default)]
struct FileDescriptorProto {
    name: Option<String>,
    package: Option<String>,
    message_types: Vec<DescriptorProto>,
}

impl FileDescriptorProto {
    /// Decodes a serialized `FileDescriptorProto` from raw bytes.
    fn parse(bytes: &[u8]) -> Result<FileDescriptorProto, ReaderError> {
        let mut reader = Reader::new(bytes);
        let entity = "FileDescriptorProto";
        let mut name = None;
        let mut package = None;
        let mut message_types = Vec::new();

        while !reader.eof() {
            let (field_number, wire_type) = reader.read_tag()?;
            match field_number {
                1 => {
                    check_wire_type(entity, "name", WireType::Len, wire_type)?;
                    let str = reader.read_string()?;
                    name = Some(str);
                }
                2 => {
                    check_wire_type(entity, "package", WireType::Len, wire_type)?;
                    let str = reader.read_string()?;
                    package = Some(str);
                }
                4 => {
                    check_wire_type(entity, "message_type", WireType::Len, wire_type)?;
                    let bytes = reader.read_len_delimited()?;
                    let message_type = DescriptorProto::parse(bytes)?;
                    message_types.push(message_type);
                }
                12 => {
                    check_wire_type(entity, "syntax", WireType::Len, wire_type)?;
                    let syntax = reader.read_string()?;
                    if syntax != "proto3" {
                        return Err(ReaderError::UnsupportedSyntax { syntax });
                    }
                }
                _ => reader.skip(wire_type)?,
            }
        }
        Ok(FileDescriptorProto {
            name,
            package,
            message_types,
        })
    }
}

/// Describes a message type.
/// See <https://github.com/protocolbuffers/protobuf/blob/v32.0/src/google/protobuf/descriptor.proto#L132>
#[derive(Debug, Default)]
pub struct DescriptorProto {
    name: Option<String>,
    fields: Vec<FieldDescriptorProto>,
    /// Nested types are used for nested types inside message and map types.
    nested_types: Vec<DescriptorProto>,
    // enum_types: Vec<EnumDescriptorProto>,
    options: Vec<MessageOption>,
}

impl DescriptorProto {
    fn parse(bytes: &[u8]) -> Result<DescriptorProto, ReaderError> {
        let mut reader = Reader::new(bytes);
        let entity = "DescriptorProto";
        let mut name = None;
        let mut fields = Vec::new();
        let mut nested_types = Vec::new();
        let mut options = Vec::new();

        while !reader.eof() {
            let (field_number, wire_type) = reader.read_tag()?;
            match field_number {
                1 => {
                    check_wire_type(entity, "name", WireType::Len, wire_type)?;
                    let str = reader.read_string()?;
                    name = Some(str);
                }
                2 => {
                    check_wire_type(entity, "field", WireType::Len, wire_type)?;
                    let bytes = reader.read_len_delimited()?;
                    let field = FieldDescriptorProto::parse(bytes)?;
                    fields.push(field);
                }
                3 => {
                    check_wire_type(entity, "nested_type", WireType::Len, wire_type)?;
                    let bytes = reader.read_len_delimited()?;
                    let message_type = DescriptorProto::parse(bytes)?;
                    nested_types.push(message_type);
                }
                7 => {
                    check_wire_type(entity, "options", WireType::Len, wire_type)?;
                    let bytes = reader.read_len_delimited()?;
                    let option = MessageOption::parse(bytes)?;
                    options.push(option);
                }
                _ => reader.skip(wire_type)?,
            }
        }
        Ok(DescriptorProto {
            name,
            fields,
            nested_types,
            options,
        })
    }
}

/// Describes an enum type.
/// See <https://github.com/protocolbuffers/protobuf/blob/v32.0/src/google/protobuf/descriptor.proto#L339>
#[derive(Debug, Default)]
struct EnumDescriptorProto {}

/// Describes option of a message type.
/// See <https://github.com/protocolbuffers/protobuf/blob/v32.0/src/google/protobuf/descriptor.proto#L581>
#[derive(Debug, Default)]
struct MessageOption {
    /// Whether the message is an automatically generated map entry type for the maps field.
    map_entry: Option<bool>,
}

impl MessageOption {
    fn parse(bytes: &[u8]) -> Result<MessageOption, ReaderError> {
        let mut reader = Reader::new(bytes);
        let entity = "MessageOption";
        let mut map_entry = None;

        while !reader.eof() {
            let (field_number, wire_type) = reader.read_tag()?;
            match field_number {
                7 => {
                    check_wire_type(entity, "map_entry", WireType::VarInt, wire_type)?;
                    let value = reader.read_bool()?;
                    map_entry = Some(value);
                }
                _ => reader.skip(wire_type)?,
            }
        }
        Ok(MessageOption { map_entry })
    }
}

#[derive(Debug, Default)]
struct FieldDescriptorProto {
    name: Option<String>,
    r#type: Option<FieldType>,
    type_name: Option<String>,
    number: Option<u32>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum FieldType {
    Double = 1,
    Float = 2,
    Int64 = 3,
    UInt64 = 4,
    Int32 = 5,
    Fixed64 = 6,
    Fixed32 = 7,
    Bool = 8,
    String = 9,
    Group = 10,
    Message = 11,
    Bytes = 12,
    UInt32 = 13,
    Enum = 14,
    SFixed32 = 15,
    SFixed64 = 16,
    SInt32 = 17,
    SInt64 = 18,
}

impl TryFrom<u64> for FieldType {
    type Error = ReaderError;

    fn try_from(value: u64) -> Result<Self, ReaderError> {
        match value {
            1 => Ok(FieldType::Double),
            2 => Ok(FieldType::Float),
            3 => Ok(FieldType::Int64),
            4 => Ok(FieldType::UInt64),
            5 => Ok(FieldType::Int32),
            6 => Ok(FieldType::Fixed64),
            7 => Ok(FieldType::Fixed32),
            8 => Ok(FieldType::Bool),
            9 => Ok(FieldType::String),
            10 => Ok(FieldType::Group),
            11 => Ok(FieldType::Message),
            12 => Ok(FieldType::Bytes),
            13 => Ok(FieldType::UInt32),
            14 => Ok(FieldType::Enum),
            15 => Ok(FieldType::SFixed32),
            16 => Ok(FieldType::SFixed64),
            17 => Ok(FieldType::SInt32),
            18 => Ok(FieldType::SInt64),
            _ => Err(ReaderError::Generic),
        }
    }
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            FieldType::Double => write!(f, "DOUBLE"),
            FieldType::Float => write!(f, "FLOAT"),
            FieldType::Int64 => write!(f, "INT64"),
            FieldType::UInt64 => write!(f, "UINT64"),
            FieldType::Int32 => write!(f, "INT32"),
            FieldType::Fixed64 => write!(f, "FIXED64"),
            FieldType::Fixed32 => write!(f, "FIXED32"),
            FieldType::Bool => write!(f, "BOOL"),
            FieldType::String => write!(f, "STRING"),
            FieldType::Group => write!(f, "GROUP"),
            FieldType::Message => write!(f, "MESSAGE"),
            FieldType::Bytes => write!(f, "BYTES"),
            FieldType::UInt32 => write!(f, "UINT32"),
            FieldType::Enum => write!(f, "ENUM"),
            FieldType::SFixed32 => write!(f, "SFIXED32"),
            FieldType::SFixed64 => write!(f, "SFIXED64"),
            FieldType::SInt32 => write!(f, "SINT32"),
            FieldType::SInt64 => write!(f, "SINT64"),
        }
    }
}

impl FieldDescriptorProto {
    fn parse(bytes: &[u8]) -> Result<FieldDescriptorProto, ReaderError> {
        let mut reader = Reader::new(bytes);
        let entity = "FieldDescriptorProto";
        let mut name = None;
        let mut field_type = None;
        let mut type_name = None;
        let mut number = None;

        while !reader.eof() {
            let (field_number, wire_type) = reader.read_tag()?;
            match field_number {
                1 => {
                    check_wire_type(entity, "name", WireType::Len, wire_type)?;
                    let value = reader.read_string()?;
                    name = Some(value);
                }
                3 => {
                    check_wire_type(entity, "number", WireType::VarInt, wire_type)?;
                    let value = reader.read_uint32()?;
                    number = Some(value);
                }
                5 => {
                    check_wire_type(entity, "type", WireType::VarInt, wire_type)?;
                    let value = reader.read_varint()?;
                    let value = FieldType::try_from(value)?;
                    field_type = Some(value);
                }
                6 => {
                    check_wire_type(entity, "type_name", WireType::Len, wire_type)?;
                    let value = reader.read_string()?;
                    type_name = Some(value);
                }

                _ => reader.skip(wire_type)?,
            }
        }

        // Check type and type_name consistency
        if let Some(field_type) = field_type
            && type_name.is_some()
        {
            // If both this and type_name are set, this must be one of TYPE_ENUM, TYPE_MESSAGE or TYPE_GROUP.
            if field_type != FieldType::Group
                && field_type != FieldType::Message
                && field_type != FieldType::Enum
            {
                panic!("Inconsistent type and type_name")
            }
        }
        Ok(FieldDescriptorProto {
            name,
            r#type: field_type,
            type_name,
            number,
        })
    }
}

fn check_wire_type(
    entity: &'static str,
    field: &'static str,
    expected_wire_type: WireType,
    actual_wire_type: WireType,
) -> Result<(), ReaderError> {
    if expected_wire_type != actual_wire_type {
        let err = ReaderError::InvalidField {
            entity: entity.to_string(),
            field: field.to_string(),
            expected_wire_type,
            actual_wire_type,
        };
        return Err(err);
    }
    Ok(())
}
