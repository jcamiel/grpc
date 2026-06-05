use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::descriptor::FileDescriptorSet;
use clap::Parser;

mod descriptor;
mod reader;
mod parser;

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
