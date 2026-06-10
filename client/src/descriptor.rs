use std::fmt;
use std::fmt::{Formatter, format};

use super::parser;
use super::parser::ParserError;
use super::reader::Reader;

/// A decoded `google.protobuf.FileDescriptorSet`.
///
/// The protocol compiler can output a FileDescriptorSet containing the .proto files it parses.
/// See <https://github.com/protocolbuffers/protobuf/blob/main/src/google/protobuf/descriptor.proto>
#[derive(Debug, Default)]
pub struct FileDescriptorSet {
    pub files: Vec<FileDescriptorProto>,
}

impl FileDescriptorSet {
    /// Decodes a serialized `FileDescriptorSet` from raw bytes.
    pub fn parse(bytes: &[u8]) -> Result<FileDescriptorSet, ParserError> {
        let mut reader = Reader::new(bytes);
        let entity = "FileDescriptorSet";
        let mut files = Vec::new();

        while !reader.eof() {
            let (field_number, wire_type) = reader.read_tag()?;
            match field_number {
                1 => {
                    let bytes = parser::message("file", entity, &mut reader, wire_type)?;
                    let file = FileDescriptorProto::parse(bytes)?;
                    files.push(file);
                }
                _ => reader.skip(wire_type)?,
            }
        }
        Ok(FileDescriptorSet { files })
    }
}

/// Describes a complete .proto file.
///
/// See <https://github.com/protocolbuffers/protobuf/blob/v32.0/src/google/protobuf/descriptor.proto#L104>
#[derive(Debug, Default)]
pub struct FileDescriptorProto {
    pub name: Option<String>,
    pub package: Option<String>,
    /// All top-level definitions in this file.
    pub message_types: Vec<DescriptorProto>,
    pub enum_types: Vec<EnumDescriptorProto>,
    pub services: Vec<ServiceDescriptorProto>,
}

impl FileDescriptorProto {
    /// Decodes a serialized `FileDescriptorProto` from raw bytes.
    fn parse(bytes: &[u8]) -> Result<FileDescriptorProto, ParserError> {
        let mut reader = Reader::new(bytes);
        let entity = "FileDescriptorProto";
        let mut name = None;
        let mut package = None;
        let mut message_types = Vec::new();
        let mut enum_types = Vec::new();
        let mut services = Vec::new();

        while !reader.eof() {
            let (field_number, wire_type) = reader.read_tag()?;
            match field_number {
                1 => {
                    let str = parser::string("name", entity, &mut reader, wire_type)?;
                    name = Some(str);
                }
                2 => {
                    let str = parser::string("package", entity, &mut reader, wire_type)?;
                    package = Some(str);
                }
                4 => {
                    let bytes = parser::message("message_type", entity, &mut reader, wire_type)?;
                    let message_type = DescriptorProto::parse(bytes)?;
                    message_types.push(message_type);
                }
                5 => {
                    let bytes = parser::message("enum_type", entity, &mut reader, wire_type)?;
                    let enum_type = EnumDescriptorProto::parse(bytes)?;
                    enum_types.push(enum_type);
                }
                6 => {
                    let bytes = parser::message("service", entity, &mut reader, wire_type)?;
                    let service = ServiceDescriptorProto::parse(bytes)?;
                    services.push(service);
                }
                12 => {
                    let syntax = parser::string("syntax", entity, &mut reader, wire_type)?;
                    if syntax != "proto3" {
                        return Err(ParserError::UnsupportedSyntax { syntax });
                    }
                }
                _ => reader.skip(wire_type)?,
            }
        }
        Ok(FileDescriptorProto {
            name,
            package,
            message_types,
            enum_types,
            services,
        })
    }
}

/// Describes a message type.
///
/// See <https://github.com/protocolbuffers/protobuf/blob/v32.0/src/google/protobuf/descriptor.proto#L151>
#[derive(Debug, Default)]
pub struct DescriptorProto {
    pub name: Option<String>,
    pub fields: Vec<FieldDescriptorProto>,
    /// Nested types are used for nested types inside message and map types.
    pub nested_types: Vec<DescriptorProto>,
    /// All enums nested within this message definition.
    pub enum_types: Vec<EnumDescriptorProto>,
    pub oneof_decls: Vec<OneOfDescriptorProto>,
    pub options: Option<MessageOptions>,
}

impl DescriptorProto {
    fn parse(bytes: &[u8]) -> Result<DescriptorProto, ParserError> {
        let mut reader = Reader::new(bytes);
        let entity = "DescriptorProto";
        let mut name = None;
        let mut fields = Vec::new();
        let mut nested_types = Vec::new();
        let mut enum_types = Vec::new();
        let mut oneof_decls = Vec::new();
        let mut options = None;

        while !reader.eof() {
            let (field_number, wire_type) = reader.read_tag()?;
            match field_number {
                1 => {
                    let str = parser::string("name", entity, &mut reader, wire_type)?;
                    name = Some(str);
                }
                2 => {
                    let bytes = parser::message("field", entity, &mut reader, wire_type)?;
                    let field = FieldDescriptorProto::parse(bytes)?;
                    fields.push(field);
                }
                3 => {
                    let bytes = parser::message("nested_type", entity, &mut reader, wire_type)?;
                    let message_type = DescriptorProto::parse(bytes)?;
                    nested_types.push(message_type);
                }
                4 => {
                    let bytes = parser::message("enum_type", entity, &mut reader, wire_type)?;
                    let enum_type = EnumDescriptorProto::parse(bytes)?;
                    enum_types.push(enum_type);
                }
                7 => {
                    let bytes = parser::message("options", entity, &mut reader, wire_type)?;
                    let value = MessageOptions::parse(bytes)?;
                    options = Some(value);
                }
                8 => {
                    let bytes = parser::message("oneof_decl", entity, &mut reader, wire_type)?;
                    let oneof_decl = OneOfDescriptorProto::parse(bytes)?;
                    oneof_decls.push(oneof_decl);
                }
                _ => reader.skip(wire_type)?,
            }
        }
        Ok(DescriptorProto {
            name,
            fields,
            nested_types,
            enum_types,
            oneof_decls,
            options,
        })
    }
}

/// Describes a field within a message.
///
/// See <https://github.com/protocolbuffers/protobuf/blob/v32.0/src/google/protobuf/descriptor.proto#L243>
#[derive(Debug, Default)]
pub struct FieldDescriptorProto {
    pub name: Option<String>,
    pub number: Option<u32>,
    pub r#type: Option<FieldType>,
    /// For message and enum types, this is the name of the type.  If the name
    /// starts with a '.', it is fully-qualified.  Otherwise, C++-like scoping
    /// rules are used to find the type (i.e. first the nested types within this
    /// message are searched, then within the parent, on up to the root
    /// namespace).
    pub type_name: Option<String>,
    pub oneof_index: Option<u32>,
    pub proto3_optional: Option<bool>,
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

impl FieldType {
    fn try_from(
        value: u64,
        field: &'static str,
        entity: &'static str,
    ) -> Result<Self, ParserError> {
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
            _ => {
                let cause = format!("Invalid enum value {} for {}:{}", value, entity, field);
                Err(ParserError::Schema { cause })
            }
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
    fn parse(bytes: &[u8]) -> Result<FieldDescriptorProto, ParserError> {
        let mut reader = Reader::new(bytes);
        let entity = "FieldDescriptorProto";
        let mut name = None;
        let mut field_type = None;
        let mut type_name = None;
        let mut number = None;
        let mut oneof_index = None;
        let mut proto3_optional = None;

        while !reader.eof() {
            let (field_number, wire_type) = reader.read_tag()?;
            match field_number {
                1 => {
                    let value = parser::string("name", entity, &mut reader, wire_type)?;
                    name = Some(value);
                }
                3 => {
                    let value = parser::uint32("number", entity, &mut reader, wire_type)?;
                    number = Some(value);
                }
                5 => {
                    let value = parser::r#enum("type", entity, &mut reader, wire_type)?;
                    let value = FieldType::try_from(value, "type", entity)?;
                    field_type = Some(value);
                }
                6 => {
                    let str = parser::string("type_name", entity, &mut reader, wire_type)?;
                    type_name = Some(str);
                }
                9 => {
                    let value = parser::uint32("oneof_index", entity, &mut reader, wire_type)?;
                    oneof_index = Some(value)
                }
                17 => {
                    // TODO: from source code
                    // When proto3_optional is true, this field must belong to a oneof to signal
                    // to old proto3 clients that presence is tracked for this field. This oneof
                    // is known as a "synthetic" oneof, and this field must be its sole member
                    let value = parser::bool("proto3_optional", entity, &mut reader, wire_type)?;
                    proto3_optional = Some(value)
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
                return Err(ParserError::Schema {
                    cause: "Inconsistent type and type_name".to_string(),
                });
            }
        }

        // TODO: check proto3_optional and oneof_index consistency

        Ok(FieldDescriptorProto {
            name,
            r#type: field_type,
            type_name,
            number,
            oneof_index,
            proto3_optional,
        })
    }
}

/// Describes an enum type.
///
/// See <https://github.com/protocolbuffers/protobuf/blob/v32.0/src/google/protobuf/descriptor.proto#L339>
#[derive(Debug, Default)]
pub struct EnumDescriptorProto {
    pub name: Option<String>,
    pub values: Vec<EnumValueDescriptorProto>,
}

impl EnumDescriptorProto {
    fn parse(bytes: &[u8]) -> Result<EnumDescriptorProto, ParserError> {
        let mut reader = Reader::new(bytes);
        let entity = "EnumDescriptorProto";
        let mut name = None;
        let mut values = Vec::new();

        while !reader.eof() {
            let (field_number, wire_type) = reader.read_tag()?;
            match field_number {
                1 => {
                    let str = parser::string("name", entity, &mut reader, wire_type)?;
                    name = Some(str);
                }
                2 => {
                    let bytes = parser::message("value", entity, &mut reader, wire_type)?;
                    let value = EnumValueDescriptorProto::parse(bytes)?;
                    values.push(value);
                }
                _ => reader.skip(wire_type)?,
            }
        }
        Ok(EnumDescriptorProto { name, values })
    }
}

/// Describes a oneof.
///
/// See <https://github.com/protocolbuffers/protobuf/blob/v32.0/src/google/protobuf/descriptor.proto#L349>
#[derive(Debug, Default)]
pub struct OneOfDescriptorProto {
    pub name: Option<String>,
}

impl OneOfDescriptorProto {
    fn parse(bytes: &[u8]) -> Result<OneOfDescriptorProto, ParserError> {
        let mut reader = Reader::new(bytes);
        let entity = "OneOfDescriptorProto";
        let mut name = None;

        while !reader.eof() {
            let (field_number, wire_type) = reader.read_tag()?;
            match field_number {
                1 => {
                    let str = parser::string("name", entity, &mut reader, wire_type)?;
                    name = Some(str);
                }
                _ => reader.skip(wire_type)?,
            }
        }
        Ok(OneOfDescriptorProto { name })
    }
}

/// Describes a value within an enum.
///
/// See <https://github.com/protocolbuffers/protobuf/blob/v32.0/src/google/protobuf/descriptor.proto#L388>
#[derive(Debug, Default)]
pub struct EnumValueDescriptorProto {
    pub name: Option<String>,
    pub number: Option<u32>,
}

impl EnumValueDescriptorProto {
    fn parse(bytes: &[u8]) -> Result<EnumValueDescriptorProto, ParserError> {
        let mut reader = Reader::new(bytes);
        let entity = "EnumValueDescriptorProto";
        let mut name = None;
        let mut number = None;

        while !reader.eof() {
            let (field_number, wire_type) = reader.read_tag()?;
            match field_number {
                1 => {
                    let str = parser::string("name", entity, &mut reader, wire_type)?;
                    name = Some(str);
                }
                2 => {
                    let value = parser::uint32("number", entity, &mut reader, wire_type)?;
                    number = Some(value);
                }
                _ => reader.skip(wire_type)?,
            }
        }
        Ok(EnumValueDescriptorProto { name, number })
    }
}

/// Describes a service.
///
/// See <https://github.com/protocolbuffers/protobuf/blob/v32.0/src/google/protobuf/descriptor.proto#L396>
#[derive(Debug, Default)]
pub struct ServiceDescriptorProto {
    pub name: Option<String>,
    pub methods: Vec<MethodDescriptorProto>,
}

impl ServiceDescriptorProto {
    fn parse(bytes: &[u8]) -> Result<ServiceDescriptorProto, ParserError> {
        let mut reader = Reader::new(bytes);
        let entity = "ServiceDescriptorProto";
        let mut name = None;
        let mut methods = Vec::new();

        while !reader.eof() {
            let (field_number, wire_type) = reader.read_tag()?;
            match field_number {
                1 => {
                    let str = parser::string("name", entity, &mut reader, wire_type)?;
                    name = Some(str);
                }
                2 => {
                    let bytes = parser::message("method", entity, &mut reader, wire_type)?;
                    let method = MethodDescriptorProto::parse(bytes)?;
                    methods.push(method);
                }
                _ => reader.skip(wire_type)?,
            }
        }
        Ok(ServiceDescriptorProto { name, methods })
    }
}

/// Describes a method of a service.
///
/// See <https://github.com/protocolbuffers/protobuf/blob/v32.0/src/google/protobuf/descriptor.proto#L404>
#[derive(Debug, Default)]
pub struct MethodDescriptorProto {
    pub name: Option<String>,
    /// Input and output type names. These are resolved in the same way as
    /// FieldDescriptorProto.type_name, but must refer to a message type.
    pub input_type: Option<String>,
    pub output_type: Option<String>,
}

impl MethodDescriptorProto {
    fn parse(bytes: &[u8]) -> Result<MethodDescriptorProto, ParserError> {
        let mut reader = Reader::new(bytes);
        let entity = "MethodDescriptorProto";
        let mut name = None;
        let mut input_type = None;
        let mut output_type = None;

        while !reader.eof() {
            let (field_number, wire_type) = reader.read_tag()?;
            match field_number {
                1 => {
                    let str = parser::string("name", entity, &mut reader, wire_type)?;
                    name = Some(str);
                }
                2 => {
                    let str = parser::string("input_type", entity, &mut reader, wire_type)?;
                    input_type = Some(str);
                }
                3 => {
                    let str = parser::string("output_type", entity, &mut reader, wire_type)?;
                    output_type = Some(str);
                }
                _ => reader.skip(wire_type)?,
            }
        }
        Ok(MethodDescriptorProto {
            name,
            input_type,
            output_type,
        })
    }
}

/// Describes option of a message type.
///
/// See <https://github.com/protocolbuffers/protobuf/blob/v32.0/src/google/protobuf/descriptor.proto#L581>
#[derive(Debug, Default)]
pub struct MessageOptions {
    /// Whether the message is an automatically generated map entry type for the maps field.
    pub map_entry: Option<bool>,
}

impl MessageOptions {
    fn parse(bytes: &[u8]) -> Result<MessageOptions, ParserError> {
        let mut reader = Reader::new(bytes);
        let entity = "MessageOption";
        let mut map_entry = None;

        while !reader.eof() {
            let (field_number, wire_type) = reader.read_tag()?;
            match field_number {
                7 => {
                    let value = parser::bool("map_entry", entity, &mut reader, wire_type)?;
                    map_entry = Some(value);
                }
                _ => reader.skip(wire_type)?,
            }
        }
        Ok(MessageOptions { map_entry })
    }
}
