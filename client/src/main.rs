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
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::descriptor::FileDescriptorSet;
use clap::Parser;

mod descriptor;
mod parser;
mod reader;

/// Rust prototype of the gRPC code paths Hurl will eventually grow.
///
/// At this stage the only thing the binary does is load a `.protoset` file
/// and hand its bytes to the decoder entry point in [`reader`].
#[derive(Parser, Debug)]
#[command(name = "client", version, about, long_about = None)]
struct Args {
    /// Path to a serialized `FileDescriptorSet`
    /// (output of `protoc --descriptor_set_out=...`, conventionally `.protoset`).
    #[arg(long, value_name = "PATH")]
    protoset: PathBuf,
}

fn main() -> ExitCode {
    let args = Args::parse();

    let bytes = match fs::read(&args.protoset) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: could not read {}: {}", args.protoset.display(), e);
            return ExitCode::FAILURE;
        }
    };

    match FileDescriptorSet::parse(&bytes) {
        Ok(fds) => {
            println!("{:#?}", fds);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: failed to decode protoset: {e}");
            ExitCode::FAILURE
        }
    }
}
