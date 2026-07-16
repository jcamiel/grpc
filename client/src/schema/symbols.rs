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
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;

use super::descriptor::{
    DescriptorProto, EnumDescriptorProto, FileDescriptorSet, MethodDescriptorProto,
    ServiceDescriptorProto,
};

/// A symbol table for all messages, enums, services of a proto definition file.
#[derive(Debug)]
pub struct SymbolTable<'fds> {
    by_fqn: HashMap<String, Symbol<'fds>>,
}

/// Represents a symbol to a message, enum or a service in a proto definition file.
#[derive(Copy, Clone, Debug)]
pub enum Symbol<'fds> {
    Message(&'fds DescriptorProto),
    Enum(&'fds EnumDescriptorProto),
    Service(&'fds ServiceDescriptorProto),
}

impl fmt::Display for Symbol<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Symbol::Message(DescriptorProto { .. }) => write!(f, "Message"),
            Symbol::Enum(EnumDescriptorProto { .. }) => write!(f, "Enum"),
            Symbol::Service(ServiceDescriptorProto { .. }) => write!(f, "Service"),
        }
    }
}

impl fmt::Display for SymbolTable<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // HashMap iteration order is non-deterministic; sort by FQN so the output is stable
        // across runs (useful for snapshot tests / diffing).
        let mut entries: Vec<(&String, &Symbol)> = self.by_fqn.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));

        writeln!(f, "SymbolTable ({} entries)", entries.len())?;
        let width = entries.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
        for (fqn, sym) in entries {
            writeln!(f, "  {:<width$} -> {}", fqn, sym, width = width)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum SymbolError {
    Duplicate { fqn: String },
}

impl fmt::Display for SymbolError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SymbolError::Duplicate { .. } => write!(f, "SymbolError::Duplicate"),
        }
    }
}

impl<'fds> SymbolTable<'fds> {
    pub fn build(fds: &'fds FileDescriptorSet) -> Result<Self, SymbolError> {
        let mut by_fqn = HashMap::new();
        for file in &fds.files {
            for msg in &file.message_types {
                add_message(&mut by_fqn, msg)?;
            }
            for enm in &file.enum_types {
                add_enum(&mut by_fqn, enm)?;
            }
            for svc in &file.services {
                add_service(&mut by_fqn, svc)?;
            }
        }
        Ok(SymbolTable { by_fqn })
    }
}

impl<'fds> SymbolTable<'fds> {
    /// Look up a service by FQN. Accepts both `.pkg.Service` (leading-dot, what `protoc` emits in
    /// `type_name` / `input_type` / `output_type`) and the dotless `pkg.Service` form.
    pub fn find_service(&self, fqn: &str) -> Option<&'fds ServiceDescriptorProto> {
        match self.by_fqn.get(normalize(fqn))? {
            Symbol::Service(svc) => Some(svc),
            _ => None,
        }
    }

    /// Look up a message by FQN. Same normalization as [`Self::find_service`].
    pub fn find_message(&self, fqn: &str) -> Option<&'fds DescriptorProto> {
        match self.by_fqn.get(normalize(fqn))? {
            Symbol::Message(msg) => Some(msg),
            _ => None,
        }
    }

    /// Look up an enum by FQN. Same normalization as [`Self::find_service`].
    pub fn find_enum(&self, fqn: &str) -> Option<&'fds EnumDescriptorProto> {
        match self.by_fqn.get(normalize(fqn))? {
            Symbol::Enum(en) => Some(en),
            _ => None,
        }
    }

    /// Look up a method on `service` by its local name.
    ///
    /// Doesn't consult the symbol table, the service descriptor already carries its methods.
    pub fn find_method(
        &self,
        service: &'fds ServiceDescriptorProto,
        method_name: &str,
    ) -> Option<&'fds MethodDescriptorProto> {
        service
            .methods
            .iter()
            .find(|m| m.name.as_deref() == Some(method_name))
    }

    /// Resolve a method's input message via its `input_type` FQN.
    pub fn resolve_method_input(&self, m: &MethodDescriptorProto) -> Option<&'fds DescriptorProto> {
        self.find_message(m.input_type.as_deref()?)
    }

    /// Resolve a method's output message via its `output_type` FQN.
    pub fn resolve_method_output(
        &self,
        m: &MethodDescriptorProto,
    ) -> Option<&'fds DescriptorProto> {
        self.find_message(m.output_type.as_deref()?)
    }
}

/// Strip the leading `.` from an absolute FQN. `protoc` emits all type references in absolute form
/// (`.pkg.Type`); the symbol table keys are stored without the dot, so we normalize at lookup time
/// and let callers pass either.
fn normalize(fqn: &str) -> &str {
    fqn.strip_prefix('.').unwrap_or(fqn)
}

/// Utilities to add entities to the hash map.
fn add_message<'fds>(
    map: &mut HashMap<String, Symbol<'fds>>,
    msg: &'fds DescriptorProto,
) -> Result<(), SymbolError> {
    insert(map, msg.fqn.clone(), Symbol::Message(msg))?;
    for nested in &msg.nested_types {
        add_message(map, nested)?;
    }
    for en in &msg.enum_types {
        add_enum(map, en)?;
    }
    Ok(())
}

fn add_enum<'fds>(
    map: &mut HashMap<String, Symbol<'fds>>,
    enm: &'fds EnumDescriptorProto,
) -> Result<(), SymbolError> {
    insert(map, enm.fqn.clone(), Symbol::Enum(enm))
}

fn add_service<'fds>(
    map: &mut HashMap<String, Symbol<'fds>>,
    svc: &'fds ServiceDescriptorProto,
) -> Result<(), SymbolError> {
    insert(map, svc.fqn.clone(), Symbol::Service(svc))
}

fn insert<'fds>(
    map: &mut HashMap<String, Symbol<'fds>>,
    fqn: String,
    sym: Symbol<'fds>,
) -> Result<(), SymbolError> {
    if map.insert(fqn.clone(), sym).is_some() {
        return Err(SymbolError::Duplicate { fqn });
    }
    Ok(())
}
