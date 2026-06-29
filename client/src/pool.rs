use std::fmt::Formatter;
use std::io;
use std::path::Path;
use std::{fmt, fs};

use crate::descriptor::FileDescriptorSet;
use crate::parser::ParserError;
use crate::symbols::{SymbolError, SymbolTable};

/// A loaded `.protoset` file. Owns the parsed descriptor set; the symbol
/// table is computed on demand and borrows from `self`.
#[derive(Debug)]
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

    /// Create a descriptor pool from a parsed `.protoset` file.
    pub fn from_descriptor_set(fds: FileDescriptorSet) -> Self {
        Self { fds }
    }

    /// Parse a `.protoset` from in-memory bytes (handy for tests).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ParserError> {
        Ok(Self {
            fds: FileDescriptorSet::parse(bytes)?,
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
