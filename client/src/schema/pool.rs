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
use std::fmt::Formatter;
use std::io;
use std::path::Path;
use std::{fmt, fs};

use super::descriptor::{FileDescriptorSet, ParserError};
use super::symbols::{SymbolError, SymbolTable};

/// A loaded `.protoset` file. Owns the parsed descriptor set; the symbol table is computed on
/// demand and borrows from `self`.
#[derive(Debug, Clone)]
pub struct DescriptorPool {
    fds: FileDescriptorSet,
}

#[derive(Debug)]
pub enum LoadError {
    Io(io::Error),
    Parse(ParserError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(err) => write!(f, "{err}"),
            LoadError::Parse(err) => write!(f, "{err}"),
        }
    }
}

impl DescriptorPool {
    /// Read and parse a `.protoset` file from disk.
    pub fn load(path: &Path) -> Result<Self, LoadError> {
        let bytes = fs::read(path).map_err(LoadError::Io)?;
        Self::from_bytes(&bytes).map_err(LoadError::Parse)
    }

    /// Create a descriptor pool from an already-parsed `FileDescriptorSet`.
    pub fn from_descriptor_set(fds: FileDescriptorSet) -> Self {
        Self { fds }
    }

    /// Parse a `.protoset` from in-memory bytes (handy for tests).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ParserError> {
        Ok(Self {
            fds: FileDescriptorSet::from(bytes)?,
        })
    }

    /// Borrow the underlying descriptor set.
    pub fn descriptor_set(&self) -> &FileDescriptorSet {
        &self.fds
    }

    /// Build a symbol table indexing the descriptor set.
    /// Cheap enough to call repeatedly (single pass over the AST).
    pub fn symbols(&self) -> Result<SymbolTable<'_>, SymbolError> {
        SymbolTable::build(&self.fds)
    }
}
