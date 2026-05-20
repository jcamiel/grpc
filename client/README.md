# Rust client prototype

This is the Rust prototype that will grow into the gRPC code paths Hurl ends up shipping. It lives outside the `hurl`
repo on purpose: this workspace is for *figuring things out*, not for code that's ready to merge.

See [`../PLAN.md`](../PLAN.md) for the staged plan. This binary corresponds to **step 5 (Descriptor-set decoder)** —
parse a `protoc`-produced `.protoset` file using our own protobuf wire-format decoder, with no third-party gRPC or
protobuf crate.

## Current state

Plumbing only — the actual decoder is a `todo!()`.

- `--protoset <PATH>` is parsed (via [`clap`](https://crates.io/crates/clap)).
- The file is read into memory.
- The bytes are handed to `protoset::decode` in [`src/protoset.rs`](src/protoset.rs).
- `decode` panics with a TODO message — to be filled in next.

`clap` is a CLI parser, not a protobuf/gRPC crate, so it doesn't breach the "no third-party gRPC or protobuf crate"
constraint stated in `PLAN.md`.

## Build & run

From this directory:

```sh
cargo build
cargo run -- --protoset ../proto/helloworld.protoset
```

Or from the repo root:

```sh
cargo run --manifest-path client/Cargo.toml -- --protoset proto/helloworld.protoset
```

Generate the `.protoset` files first (see [`../server/README.md`](../server/README.md) step 4).

## What's next

Implement `protoset::decode` against the protobuf wire format:

- Varint encode/decode (tags, lengths, ints, bools, enums).
- Fixed 32-bit and 64-bit little-endian.
- Length-delimited (strings, bytes, embedded messages, packed repeated).
- Field tag = `(field_number << 3) | wire_type`.
- ZigZag for `sint32` / `sint64`.

The relevant parts of `descriptor.proto` (`FileDescriptorSet` → `FileDescriptorProto` → `DescriptorProto`,
`FieldDescriptorProto`, `ServiceDescriptorProto`, `MethodDescriptorProto`, `EnumDescriptorProto`) are the only fields
we need to interpret — everything else can be skipped as unknown and discarded.

## References

- **Protobuf encoding spec** — <https://protobuf.dev/programming-guides/encoding>
- `descriptor.proto` (Google's schema for descriptors) —
  <https://github.com/protocolbuffers/protobuf/blob/main/src/google/protobuf/descriptor.proto>
- gRPC-over-HTTP/2 protocol — <https://github.com/grpc/grpc/blob/master/doc/PROTOCOL-HTTP2.md>
