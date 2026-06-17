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

use super::descriptor::{DescriptorProto, FileDescriptorSet, ServiceDescriptorProto};

/// A symbol table for all messages, enums, services of a proto definition file.
#[derive(Debug)]
pub struct SymbolTable<'fds> {
    by_fqn: HashMap<String, Symbol<'fds>>,
}

/// Represents a symbol to a message, enum or a service in a proto definition file.
#[derive(Copy, Clone, Debug)]
pub enum Symbol<'fds> {
    Message(&'fds DescriptorProto),
    Service(&'fds ServiceDescriptorProto),
}

impl fmt::Display for Symbol<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Symbol::Message(DescriptorProto { .. }) => write!(f, "Message"),
            Symbol::Service(ServiceDescriptorProto { .. }) => write!(f, "Service"),
        }
    }
}

impl fmt::Display for SymbolTable<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // HashMap iteration order is non-deterministic; sort by FQN so the
        // output is stable across runs (useful for snapshot tests / diffing).
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
            let pkg = file.package.as_deref().unwrap_or("");
            for msg in &file.message_types {
                add_message(&mut by_fqn, pkg, msg)?;
            }
            for svc in &file.services {
                add_service(&mut by_fqn, pkg, svc)?;
            }
        }
        Ok(SymbolTable { by_fqn })
    }
}

/// Utilities to add entities to the hash map.

fn join(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}.{name}")
    }
}

fn add_message<'fds>(
    map: &mut HashMap<String, Symbol<'fds>>,
    parent: &str,
    msg: &'fds DescriptorProto,
) -> Result<(), SymbolError> {
    let Some(name) = &msg.name else { return Ok(()) };
    let fqn = join(parent, name);
    insert(map, fqn.clone(), Symbol::Message(msg))?;
    for nested in &msg.nested_types {
        add_message(map, &fqn, nested)?;
    }
    // for en     in &msg.enum_types   { add_enum   (map, &fqn, en    )?; }
    Ok(())
}

fn add_service<'fds>(
    map: &mut HashMap<String, Symbol<'fds>>,
    parent: &str,
    svc: &'fds ServiceDescriptorProto,
) -> Result<(), SymbolError> {
    let Some(name) = &svc.name else { return Ok(()) };
    let svc_fqn = join(parent, name);
    insert(map, svc_fqn.clone(), Symbol::Service(svc))?;
    Ok(())
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
