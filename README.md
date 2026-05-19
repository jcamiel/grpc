# Study / POC for gRPC support in Hurl

This repository is a research and proof-of-concept workspace for adding **gRPC support to [Hurl](https://hurl.dev)**.

The goal is not to ship production code from this repo, but to explore the problem space, prototype solutions, and
answer the open design questions before the work lands in `hurl`.

## Scope

**Unary calls only — for now.** One request, one optional response, matching the classic Hurl entry shape (one request
+ one optional response per entry). Server-streaming, client-streaming, and bidirectional streaming are explicitly out
of scope for this POC. We can revisit streaming once unary lands and we have a feel for the syntax and output format.

## What we want to figure out

- **Landscape** — How do other HTTP/CLI clients support gRPC today (grpcurl, evans, Postman, Insomnia, BloomRPC, Kreya,
  curl, `buf curl`, etc.)? What works well for *unary* calls, and what should we copy or avoid?
- **Hurl syntax** — What would a natural Hurl-flavored syntax for a gRPC unary call look like? How do we describe the
  service/method, the request message, the metadata, and the expected response in a `.hurl` file?
- **Options & CLI flags** — What new request options, asserts, and CLI flags are needed (e.g. `--proto`,
  `--proto-path`, `--import-path`, `--reflection`, per-request `[Options]` entries)?
- **Protobuf version** — proto2 vs proto3. Which do we target first? How do we handle `Any`, `oneof`, `map`, well-known
  types, optional/required, defaults?
- **Output** — What should Hurl print when gRPC is enabled? How do we render the response message, trailers, and
  `grpc-status` in a way that stays consistent with Hurl's existing HTTP output (`--verbose`, `-i`, `--json`, etc.)?
- **Reference server** — A small Python gRPC server lives in this repo so we have something realistic to point both
  the prototype *and other gRPC clients* at while iterating on syntax and output. This is the first piece we build.
- **Zero third-party crates** — The hard constraint: can we add gRPC to Hurl **without pulling in `tonic`, `prost`,
  `protobuf`, or any other gRPC/protobuf crate**? That means parsing `.proto` files (or descriptor sets) ourselves,
  encoding/decoding the protobuf wire format ourselves, and speaking gRPC framing on top of the HTTP/2 transport Hurl
  already gets through libcurl.

## Repository layout

```
.
├── README.md         # this file — what & why
├── PLAN.md           # the plan — how & in what order
└── (to come)
    ├── server/       # Python reference gRPC server (built first)
    ├── proto/        # .proto files used by the server and the POC
    ├── samples/      # example .hurl files exercising the proposed syntax
    └── poc/          # Rust prototype of the protobuf + gRPC code paths
```

See [PLAN.md](./PLAN.md) for the staged plan and the open questions we want to close out.
