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

use super::resolve;
use crate::wire::WireType;
use crate::wire::reader::{Reader, ReaderError};

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

/// A decoded `google.protobuf.FileDescriptorSet`.
///
/// The protocol compiler can output a FileDescriptorSet containing the .proto files it parses.
/// See <https://github.com/protocolbuffers/protobuf/blob/main/src/google/protobuf/descriptor.proto>
#[derive(Clone, Debug, Default)]
pub struct FileDescriptorSet {
    pub files: Vec<FileDescriptorProto>,
}

impl FileDescriptorSet {
    /// Decodes a serialized `FileDescriptorSet` from raw bytes.
    ///
    /// Runs the wire decoder first (scope-agnostic), then a single [`resolve::resolve_fqns`] pass
    /// to fill in every `fqn` field top-down. The returned set has fully-resolved FQNs on every
    /// named descriptor.
    pub fn from(bytes: &[u8]) -> Result<FileDescriptorSet, ParserError> {
        let mut set = Self::parse(bytes)?;
        resolve::resolve_fqns(&mut set);
        Ok(set)
    }

    /// Decodes a serialized `FileDescriptorSet` from raw bytes, without resolving fully-qualified-name.
    ///
    /// The pure wire-decoding half of [`Self::from`], with no scope resolution. Descriptors returned
    /// from here have `fqn` at its default (empty string).
    fn parse(bytes: &[u8]) -> Result<FileDescriptorSet, ParserError> {
        let mut reader = Reader::new(bytes);
        let entity = "FileDescriptorSet";
        let mut files = Vec::new();

        while !reader.eof() {
            let (field_number, wire_type) = reader.read_tag()?;
            match field_number {
                1 => {
                    let bytes = message("file", entity, &mut reader, wire_type)?;
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
#[derive(Clone, Debug, Default)]
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
                    let str = string("name", entity, &mut reader, wire_type)?;
                    name = Some(str);
                }
                2 => {
                    let str = string("package", entity, &mut reader, wire_type)?;
                    package = Some(str);
                }
                4 => {
                    let bytes = message("message_type", entity, &mut reader, wire_type)?;
                    let message_type = DescriptorProto::parse(bytes)?;
                    message_types.push(message_type);
                }
                5 => {
                    let bytes = message("enum_type", entity, &mut reader, wire_type)?;
                    let enum_type = EnumDescriptorProto::parse(bytes)?;
                    enum_types.push(enum_type);
                }
                6 => {
                    let bytes = message("service", entity, &mut reader, wire_type)?;
                    let service = ServiceDescriptorProto::parse(bytes)?;
                    services.push(service);
                }
                12 => {
                    let syntax = string("syntax", entity, &mut reader, wire_type)?;
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
#[derive(Clone, Debug, Default)]
pub struct DescriptorProto {
    pub name: Option<String>,
    /// Fully-qualified name (e.g. `echo.Payload`, `echo.Payload.Priority`), computed at parse time
    /// from the enclosing scope's FQN + this message's local `name`. Not present on the wire.
    pub fqn: String,
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
                    let str = string("name", entity, &mut reader, wire_type)?;
                    name = Some(str);
                }
                2 => {
                    let bytes = message("field", entity, &mut reader, wire_type)?;
                    let field = FieldDescriptorProto::parse(bytes)?;
                    fields.push(field);
                }
                3 => {
                    let bytes = message("nested_type", entity, &mut reader, wire_type)?;
                    let message_type = DescriptorProto::parse(bytes)?;
                    nested_types.push(message_type);
                }
                4 => {
                    let bytes = message("enum_type", entity, &mut reader, wire_type)?;
                    let enum_type = EnumDescriptorProto::parse(bytes)?;
                    enum_types.push(enum_type);
                }
                7 => {
                    let bytes = message("options", entity, &mut reader, wire_type)?;
                    let value = MessageOptions::parse(bytes)?;
                    options = Some(value);
                }
                8 => {
                    let bytes = message("oneof_decl", entity, &mut reader, wire_type)?;
                    let oneof_decl = OneOfDescriptorProto::parse(bytes)?;
                    oneof_decls.push(oneof_decl);
                }
                _ => reader.skip(wire_type)?,
            }
        }

        Ok(DescriptorProto {
            name,
            fqn: String::new(),
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
#[derive(Clone, Debug, Default)]
pub struct FieldDescriptorProto {
    pub name: Option<String>,
    pub number: Option<u32>,
    pub label: Option<FieldLabel>,
    pub r#type: Option<FieldType>,
    /// For message and enum types, this is the name of the type.  If the name starts with a '.',
    /// it is fully-qualified.  Otherwise, C++-like scoping rules are used to find the type
    /// (i.e. first the nested types within this message are searched, then within the parent,
    /// on up to the root namespace).
    pub type_name: Option<String>,
    pub oneof_index: Option<u32>,
    pub proto3_optional: Option<bool>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FieldType {
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
    /// Does this field have **explicit presence** in proto3? i.e. can a decoder distinguish "field
    /// was sent (even at its default value)" from "no field with this tag appeared on the wire"?
    ///
    /// Three sources of explicit presence in proto3:
    /// 1. Singular message-typed fields — always.
    /// 2. Fields inside a `oneof` — covers both user-declared oneofs and the synthetic single-member
    ///    oneof that `optional` generates.
    /// 3. Nothing else. Bare scalars / enums, repeated, and map fields have **implicit** presence:
    ///    default value is indistinguishable from unset.
    pub fn has_explicit_presence(&self) -> bool {
        // The `label == Repeated` check must come first, because in the descriptor form a `map<K, V>`
        // field looks like a `repeated <Entry>` message; checking `type == Message` first would misidentify
        // maps as having presence.
        if self.label == Some(FieldLabel::Repeated) {
            return false;
        }
        if self.r#type == Some(FieldType::Message) {
            return true;
        }
        // Optional field with explicit presence.
        if self.proto3_optional == Some(true) {
            return true;
        }
        if self.oneof_index.is_some() {
            return true;
        }
        false
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FieldLabel {
    Optional = 1,
    Repeated = 3,
    // The required label should not be supported as we target protobuf3 syntax.
    Required = 2,
}

impl FieldLabel {
    fn try_from(
        value: u64,
        field: &'static str,
        entity: &'static str,
    ) -> Result<Self, ParserError> {
        match value {
            1 => Ok(FieldLabel::Optional),
            2 => Ok(FieldLabel::Required),
            3 => Ok(FieldLabel::Repeated),
            _ => {
                let cause = format!("Invalid enum value {} for {}:{}", value, entity, field);
                Err(ParserError::Schema { cause })
            }
        }
    }
}

impl fmt::Display for FieldLabel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            FieldLabel::Optional => write!(f, "OPTIONAL"),
            FieldLabel::Required => write!(f, "REQUIRED"),
            FieldLabel::Repeated => write!(f, "REPEATED"),
        }
    }
}

impl FieldDescriptorProto {
    fn parse(bytes: &[u8]) -> Result<FieldDescriptorProto, ParserError> {
        let mut reader = Reader::new(bytes);
        let entity = "FieldDescriptorProto";
        let mut name = None;
        let mut number = None;
        let mut label = None;
        let mut field_type = None;
        let mut type_name = None;
        let mut oneof_index = None;
        let mut proto3_optional = None;

        while !reader.eof() {
            let (field_number, wire_type) = reader.read_tag()?;
            match field_number {
                1 => {
                    let value = string("name", entity, &mut reader, wire_type)?;
                    name = Some(value);
                }
                3 => {
                    let value = uint32("number", entity, &mut reader, wire_type)?;
                    number = Some(value);
                }
                4 => {
                    let value = r#enum("label", entity, &mut reader, wire_type)?;
                    let value = FieldLabel::try_from(value, "label", entity)?;
                    label = Some(value);
                }
                5 => {
                    let value = r#enum("type", entity, &mut reader, wire_type)?;
                    let value = FieldType::try_from(value, "type", entity)?;
                    field_type = Some(value);
                }
                6 => {
                    let str = string("type_name", entity, &mut reader, wire_type)?;
                    type_name = Some(str);
                }
                9 => {
                    let value = uint32("oneof_index", entity, &mut reader, wire_type)?;
                    oneof_index = Some(value)
                }
                17 => {
                    // TODO: from source code
                    // When proto3_optional is true, this field must belong to a oneof to signal
                    // to old proto3 clients that presence is tracked for this field. This oneof
                    // is known as a "synthetic" oneof, and this field must be its sole member
                    let value = bool("proto3_optional", entity, &mut reader, wire_type)?;
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

        // TODO: check label and required

        Ok(FieldDescriptorProto {
            name,
            number,
            label,
            r#type: field_type,
            type_name,
            oneof_index,
            proto3_optional,
        })
    }
}

/// Describes an enum type.
///
/// See <https://github.com/protocolbuffers/protobuf/blob/v32.0/src/google/protobuf/descriptor.proto#L339>
#[derive(Clone, Debug, Default)]
pub struct EnumDescriptorProto {
    pub name: Option<String>,
    /// Fully-qualified name.
    pub fqn: String,
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
                    let str = string("name", entity, &mut reader, wire_type)?;
                    name = Some(str);
                }
                2 => {
                    let bytes = message("value", entity, &mut reader, wire_type)?;
                    let value = EnumValueDescriptorProto::parse(bytes)?;
                    values.push(value);
                }
                _ => reader.skip(wire_type)?,
            }
        }
        Ok(EnumDescriptorProto {
            name,
            fqn: String::new(),
            values,
        })
    }
}

/// Describes a oneof.
///
/// See <https://github.com/protocolbuffers/protobuf/blob/v32.0/src/google/protobuf/descriptor.proto#L349>
#[derive(Clone, Debug, Default)]
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
                    let str = string("name", entity, &mut reader, wire_type)?;
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
#[derive(Clone, Debug, Default)]
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
                    let str = string("name", entity, &mut reader, wire_type)?;
                    name = Some(str);
                }
                2 => {
                    let value = uint32("number", entity, &mut reader, wire_type)?;
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
#[derive(Clone, Debug, Default)]
pub struct ServiceDescriptorProto {
    pub name: Option<String>,
    /// Fully-qualified name (e.g. `helloworld.Greeter`).
    pub fqn: String,
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
                    let str = string("name", entity, &mut reader, wire_type)?;
                    name = Some(str);
                }
                2 => {
                    let bytes = message("method", entity, &mut reader, wire_type)?;
                    methods.push(MethodDescriptorProto::parse(bytes)?);
                }
                _ => reader.skip(wire_type)?,
            }
        }
        Ok(ServiceDescriptorProto {
            name,
            fqn: String::new(),
            methods,
        })
    }
}

/// Describes a method of a service.
///
/// See <https://github.com/protocolbuffers/protobuf/blob/v32.0/src/google/protobuf/descriptor.proto#L404>
#[derive(Clone, Debug, Default)]
pub struct MethodDescriptorProto {
    pub name: Option<String>,
    /// Fully-qualified name (e.g. `helloworld.Greeter.SayHello`).
    pub fqn: String,
    /// Input and output type names. These are resolved in the same way as
    /// FieldDescriptorProto.type_name, but must refer to a message type.
    pub input_type: Option<String>,
    pub output_type: Option<String>,
    /// Identifies if client streams multiple client messages
    pub client_streaming: Option<bool>,
    /// Identifies if server streams multiple server messages
    pub server_streaming: Option<bool>,
}

impl MethodDescriptorProto {
    fn parse(bytes: &[u8]) -> Result<MethodDescriptorProto, ParserError> {
        let mut reader = Reader::new(bytes);
        let entity = "MethodDescriptorProto";
        let mut name = None;
        let mut input_type = None;
        let mut output_type = None;
        let mut client_streaming = None;
        let mut server_streaming = None;

        while !reader.eof() {
            let (field_number, wire_type) = reader.read_tag()?;
            match field_number {
                1 => {
                    let str = string("name", entity, &mut reader, wire_type)?;
                    name = Some(str);
                }
                2 => {
                    let str = string("input_type", entity, &mut reader, wire_type)?;
                    input_type = Some(str);
                }
                3 => {
                    let str = string("output_type", entity, &mut reader, wire_type)?;
                    output_type = Some(str);
                }
                5 => {
                    let value = bool("client_streaming", entity, &mut reader, wire_type)?;
                    client_streaming = Some(value);
                }
                6 => {
                    let value = bool("server_streaming", entity, &mut reader, wire_type)?;
                    server_streaming = Some(value);
                }
                _ => reader.skip(wire_type)?,
            }
        }
        Ok(MethodDescriptorProto {
            name,
            fqn: String::new(),
            input_type,
            output_type,
            client_streaming,
            server_streaming,
        })
    }
}

/// Describes option of a message type.
///
/// See <https://github.com/protocolbuffers/protobuf/blob/v32.0/src/google/protobuf/descriptor.proto#L581>
#[derive(Clone, Debug, Default)]
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
                    let value = bool("map_entry", entity, &mut reader, wire_type)?;
                    map_entry = Some(value);
                }
                _ => reader.skip(wire_type)?,
            }
        }
        Ok(MessageOptions { map_entry })
    }
}

// Helpers methods

fn string(
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

fn bool(
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

fn uint32(
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

fn message<'input>(
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

fn r#enum(
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
