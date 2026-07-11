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
mod client;
mod request;
mod schema;
mod wire;

use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use url::Url;

use client::Client;
use schema::pool::DescriptorPool;

/// Rust prototype of the gRPC code paths Hurl will eventually grow.
///
/// At this stage the only thing the binary does is load a `.protoset` file
/// and hand its bytes to the decoder entry point in [`reader`].
#[derive(Parser, Debug)]
#[command(name = "client", version, about, long_about = None)]
struct Args {
    url: Option<String>,
    /// Path to a serialized `FileDescriptorSet`
    /// (output of `protoc --descriptor_set_out=...`, conventionally `.protoset`).
    #[arg(long, value_name = "PATH")]
    protoset: PathBuf,
    /// Request body as a JSON string. If omitted, the body is read from standard input.
    #[arg(short = 'd', long, value_name = "STRING")]
    data: Option<String>,
}

fn main() -> ExitCode {
    let args = Args::parse();

    let protoset = Path::new(&args.protoset);
    let descriptor_pool = match DescriptorPool::load(protoset) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: could not load {}: {:#?}", protoset.display(), e);
            return ExitCode::FAILURE;
        }
    };
    println!("{:#?}", descriptor_pool);

    // Without a URL there is nothing to send: print the decoded pool and exit.
    let Some(url) = args.url else {
        return ExitCode::SUCCESS;
    };

    // Request body: use `--data` if provided, otherwise read from standard input.
    let body = match args.data {
        Some(data) => data,
        None => {
            let mut buf = String::new();
            if let Err(e) = io::stdin().read_to_string(&mut buf) {
                eprintln!("error: could not read body from stdin: {e}");
                return ExitCode::FAILURE;
            }
            buf
        }
    };

    let client = Client::new();
    let url = Url::parse(&url).unwrap();
    let _r = match client.run(descriptor_pool, url, body.as_bytes()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    ExitCode::SUCCESS
}
