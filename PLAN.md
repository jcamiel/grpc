# Plan — gRPC support in Hurl

This is the working plan for the POC. It is a living document: every section ends with **open questions** that we want
to answer before committing to a design in `hurl` proper.

Two constraints shape everything below:

> **Unary only.** One request, one optional response — the classic Hurl shape. No server-streaming, client-streaming,
> or bidi in this POC.
>
> **No third-party gRPC or protobuf crate.** We implement the wire format and gRPC framing ourselves in Rust.

---

## 1. Python reference server (first)

We build this **before** anything else. The reason is practical: we want a real, running gRPC endpoint to point *other
clients* (grpcurl, evans, Postman, `buf curl`, plain `curl --http2`) at while we do the survey in section 2. Without
the server, the survey turns into reading docs; with it, we can actually run each tool and see what they produce.

Python is chosen because the official `grpcio` tooling is simple and the server is throwaway scaffolding — it does
**not** count against the "no third-party crates" constraint, which applies only to the Rust side.

**Scope of the server (unary only)**

- A small set of unary RPCs that together cover the interesting wire-format cases:
  - `Greeter.SayHello(HelloRequest) → HelloReply` — the trivial baseline.
  - `Echo.Echo(EchoRequest) → EchoReply` — a message that exercises every wire-format case we care about.
  - `Status.Fail(FailRequest) → FailReply` — deliberately returns specific `grpc-status` codes / `grpc-message` values
    so we can drive error rendering and assertions.
- One "kitchen-sink" message type used by `Echo` that exercises:
  - scalar fields of every wire type (varint, 64-bit, length-delimited, 32-bit)
  - `repeated` (packed and unpacked)
  - nested messages
  - `oneof`
  - `map<string, ...>`
  - `enum`
  - well-known types: `Timestamp`, `Duration`, `Any` (at least one)
- Plaintext endpoint on a fixed port.
- Optional TLS endpoint (self-signed) so we can exercise the TLS path too.
- Server reflection enabled, so we can later compare a `.proto`-driven flow against a reflection-driven flow.

**Deliverable**: `server/` with a `make run` (or `python -m server`) entry point and a fixed port, plus a `proto/`
directory with the `.proto` files. Should be one-command-to-run from a clean checkout.

**Open questions**

- Do we want a deterministic-output mode (e.g. fixed timestamps, no random IDs) so the POC's golden output stays
  stable?
- Do we add a slow-response endpoint to exercise client timeouts?

---

## 2. Survey other gRPC clients

With the server running, we point each tool at it and write up what we see. The output of this phase is a short
comparison note (can live in this repo as `notes/clients.md`).

| Tool       | Style            | Schema source                    | Notes to capture                             |
|------------|------------------|----------------------------------|----------------------------------------------|
| `grpcurl`  | CLI, curl-like   | `.proto`, descriptor, reflection | The de-facto baseline. Output format, flags. |
| `evans`    | REPL + CLI       | `.proto`, reflection             | Interactive UX — not our model but useful.   |
| Postman    | GUI              | `.proto`, reflection             | How they present unary responses.            |
| Insomnia   | GUI              | `.proto`, reflection             | Same.                                        |
| BloomRPC   | GUI (deprecated) | `.proto`                         | Historical reference.                        |
| Kreya      | GUI              | `.proto`, reflection             | Modern UX cues.                              |
| `curl`     | CLI              | none (raw bytes)                 | What you *can* do with just HTTP/2.          |
| `buf curl` | CLI              | `.proto`, reflection, BSR        | Modern grpcurl alternative.                  |

Streaming RPC support in these tools is interesting but **out of scope** for the survey — we only look at how each
handles unary.

**For each tool, capture:**

- How is the service/method addressed? (`pkg.Service/Method`, URL form, etc.)
- How is the request body provided? (JSON on stdin, JSON arg, file, etc.)
- How are headers/metadata expressed?
- How is the response rendered? (JSON, pretty proto, raw?)
- How are trailers and `grpc-status` surfaced?
- How is TLS / plaintext / authority controlled?

**Open questions**

- Which tool's UX is closest in spirit to Hurl's "plain text, scriptable, no GUI" stance? `grpcurl` and `buf curl` are
  the obvious candidates.
- Is there prior art for *asserting* on gRPC responses the way Hurl asserts on HTTP responses?

---

## 3. Proposed Hurl syntax

This is the part that needs the most iteration. The deliverable is one or two candidate syntaxes written up as
runnable-looking `.hurl` files in `samples/`, plus a rationale.

**Candidate A — POST + fenced `protobuf` body (preferred)**

~~~hurl
POST localhost:50051/helloworld.Greeter/SayHello
[Options]
proto: proto/helloworld.proto
```protobuf
{
  "name": "World"
}
```
HTTP 200
[Asserts]
header "grpc-status" toInt == 0
jsonpath "$.message" == "Hello, World"
~~~

- Pros: no new verb and no `grpc: true` flag — the `` ```protobuf `` fence is the one and only signal that this body
  gets serialized to protobuf wire bytes and the entry is gRPC. The fence labels *what goes on the wire*, which is
  what a reader actually needs to know. Natural extension point for future body forms (e.g. `` ```prototext ``).
- Cons: the entry's source diverges slightly from a plain HTTP POST (the fence is mandatory); requires Hurl to learn
  that a fenced-language body can drive transport behavior.

**Candidate B — dedicated `GRPC` verb**

```hurl
GRPC localhost:50051/helloworld.Greeter/SayHello
grpc-timeout: 5S
authorization: Bearer xyz
{
  "name": "World"
}
HTTP 200
[Asserts]
header "grpc-status" toInt == 0
jsonpath "$.message" == "Hello, World"
```

- Pros: explicit at a glance — a new top-level verb is hard to miss; room for gRPC-specific headers/asserts.
- Cons: introduces a new verb when gRPC *is* HTTP/2 POST under the hood; less consistent with how Hurl currently
  models requests.

**Cross-cutting design decisions to settle**

- **Schema source** — `[Options] proto: ...` per request? a CLI `--proto`? `--proto-path` for imports? `--reflection`
  to skip `.proto` entirely?
- **Metadata** — gRPC metadata is HTTP/2 headers. Reuse Hurl's existing header section, but document the `grpc-`
  reserved names.
- **Trailers & status** — `grpc-status`, `grpc-message`, and `grpc-status-details-bin` arrive as HTTP/2 trailers. If
  libcurl surfaces trailers in the same channel as response headers (see §6.1), we do **not** need new query types
  for them — Hurl's existing `header "<name>"` query (combined with filters like `toInt`) handles all three:
  `header "grpc-status" toInt == 0`, `header "grpc-message" contains "..."`, etc.
- **Response body in asserts** — Symmetric with the request: a `` ```protobuf `` block in the response section, with
  expected JSON-shaped content inside, decoded by Hurl before assertions run. `jsonpath` queries then operate on the
  decoded view. (Open: do we also want a `protopath` query type, or is JSON enough?)

**Open questions**

- Do we ever need Candidate B at all? Candidate A handles everything via the fenced body; Candidate B would only earn
  its keep if we discover something gRPC-specific that doesn't fit in `[Options]` or in the headers.
- Do we want `GRPCS` / TLS handling via URL scheme, or always rely on a CLI flag / option?
- Do we need any new query types at all? Trailers reuse `header`, response messages reuse `jsonpath` on the decoded
  view — a dedicated `protopath` would only be worth it if `jsonpath` can't express something we care about.

---

## 4. Options and CLI flags

With Candidate A, the `` ```protobuf `` body fence is the only signal that an entry is gRPC, so we **don't** need a
`--grpc` flag, a per-request `grpc: true` option, or a `--grpc-format` switch. The remaining options are all about
*where the schema comes from* and a small bit of HTTP/2 plumbing.

- `--proto <path>` — single `.proto` file.
- `--proto-path <dir>` — repeatable, like protoc's `-I`.
- `--reflection` — use server reflection instead of local `.proto`.
- Per-request `[Options]`:
  - `proto: <path>` (overrides / supplements CLI `--proto`)
  - `grpc-authority: <host>` (override `:authority`)

**Open questions**

- If an entry has a `` ```protobuf `` body but no `--proto` / `proto:` / `--reflection` is provided, do we error or
  attempt reflection by default? Probably error — too easy to silently hit the wrong server otherwise.
- How do we discover the service/method when only `--reflection` is given? The URL still names the method, but we
  need the schema to transcode the body.

---

## 5. Output format

When gRPC is enabled, `hurl` and `hurl --verbose` need to render:

- The HTTP/2 request line (`POST /pkg.Service/Method HTTP/2`).
- Request headers including `content-type: application/grpc+proto`, `te: trailers`.
- The encoded request frame size (and JSON-decoded view in `--verbose`).
- The response status, headers, and **trailers** — trailers are the part HTTP output today does not show, and gRPC
  depends on them.
- `--json` mode: a stable JSON shape that includes `grpcStatus`, `grpcMessage`, the decoded response message, and the
  raw bytes (base64) so users can post-process.

**Open questions**

- Do we always decode to JSON, or keep a `--raw` escape hatch?
- How do we render binary fields (`bytes`) in the default human output? Hex preview + length, like Hurl does for
  binary HTTP bodies, is the natural fit.

---

## 6. The hard part — no third-party crates

This is the section the whole repo is really about. We need to validate that each of these layers can be implemented
in plain Rust, in a reasonable amount of code, without `tonic` / `prost` / `protobuf` / `h2` / `prost-build`.

### 6.1 HTTP/2 transport

Hurl talks HTTP through libcurl. libcurl supports HTTP/2 already.

- Verify: can libcurl emit the exact frames gRPC needs (HEADERS with `:method POST`, `content-type: application/grpc`,
  `te: trailers`; DATA with length-prefixed messages; and read response **trailers**)?
- Verify: can we read trailers from libcurl in the Rust binding Hurl uses? This is the riskiest libcurl-side question.
- **Critical:** ideally libcurl surfaces trailers in the same channel as response headers — if it does, §3's asserts
  story is essentially free (the existing `header "<name>"` query handles `grpc-status`, `grpc-message`, etc. with no
  new query types). If trailers come out of a separate API, we either need to introduce a `trailer "<name>"` query
  or splice them into the headers list ourselves before assertions run.

**Open questions**

- If libcurl can't surface trailers cleanly, what's the fallback? (A: read raw HTTP/2 frames ourselves — much more
  work; B: a tiny dependency just for trailer extraction.)

### 6.2 gRPC framing

Trivial on paper:

```
+--+----+--------+
|0 |len |payload |   compressed-flag (1 byte) | length (4 byte BE) | bytes
+--+----+--------+
```

- Implement encode and decode for the length-prefixed message frame.
- Implement the compression flag (we can punt on actually supporting gzip in v1 — just reject non-zero with a clear
  error).

Since we're unary-only, there is at most one frame in each direction, which simplifies the buffering story
considerably.

**Open questions**

- Do we support `grpc-encoding: gzip` in v1, or defer?

### 6.3 Protobuf wire format

The wire format is small and stable. We need:

- Varint encode/decode (used for tags, lengths, and int/bool/enum fields).
- ZigZag for `sint32`/`sint64`.
- Fixed 32-bit and 64-bit little-endian.
- Length-delimited (strings, bytes, embedded messages, packed repeated).
- Field tag = `(field_number << 3) | wire_type`.
- Unknown fields: preserve on decode (gRPC requires forward-compat behavior).

This is genuinely small — a few hundred lines of Rust. The POC in `poc/` should prove this end-to-end against the
Python server.

**Open questions**

- Do we model messages as a typed AST (driven by the parsed `.proto`) or as a schemaless `Map<u32, Value>`? The typed
  approach gives better errors and JSON conversion; the schemaless one is smaller but pushes work onto the query
  layer.

### 6.4 `.proto` file parsing

This is the biggest unknown. We need to parse enough of the proto3 grammar to:

- Resolve `service` blocks → list of methods with input/output type names.
- Resolve `message` blocks → field number, name, type, label, `oneof`, `map`.
- Resolve `enum` blocks.
- Follow `import` statements across `--proto-path` directories.
- Handle the standard well-known types we care about (`Timestamp`, `Duration`, `Any`, `Empty`, `Struct` at minimum).

**Realistic scope for v1**

- proto3 only.
- No `extend` (proto2-only anyway).
- No custom options / no `option (...)` evaluation beyond ignoring them.
- We still need to *parse* the `stream` keyword on RPCs (so `.proto` files containing streaming methods don't fail to
  load), but we **reject** streaming methods at call time with a clear error.

**Open questions**

- Do we hand-write a recursive-descent parser, or write a tiny generic tokenizer + parser combinator? Hand-written is
  probably less code overall.
- Do we accept *descriptor sets* (`protoc -o file.pb`) as an alternative input? Parsing a `FileDescriptorSet` is just
  protobuf decoding (which we already have) and sidesteps `.proto` text parsing. **This might be the right v1.**

### 6.5 Server reflection

The gRPC server reflection protocol is itself a gRPC service. Once 6.1–6.4 work, reflection is "free" in the sense
that we already have all the machinery. But it pulls in a fixed `.proto` (`grpc.reflection.v1.ServerReflection`) that
we'd need to either bundle as text or as a precomputed descriptor.

Reflection's own RPCs are streaming in the official `.proto`, but for unary schema lookup we can issue a single
request/response in practice. If that turns out to require true bidi support, reflection slips to a later phase.

**Open questions**

- v1 with `.proto` only, reflection deferred?
- Or v1 with reflection only (no `.proto` parsing!) — leaning on descriptor decoding, which we have to write anyway?
  This could be a *very* small v1.
- Is the reflection service usable as effectively-unary, or does it really need bidi?

---

## 7. Staged delivery

Roughly in this order, each stage gated on the previous one:

1. **Python server + protos** (`server/`, `proto/`) — first, so we have something to point other clients at.
2. **Survey notes** (`notes/clients.md`) — run grpcurl / evans / `buf curl` etc. against the server and write up the
   unary UX of each.
3. **Wire-format prototype** in `poc/`: encode/decode a hand-crafted message against the Python server, no `.proto`
   parsing yet — hard-coded message shape.
4. **gRPC framing + libcurl trailers** — verify the transport story end to end with the hard-coded message, unary
   only.
5. **Descriptor-set decoder** — parse a `protoc`-produced `.pb` file using our own protobuf decoder. This gives us a
   full schema without writing a `.proto` parser yet.
6. **Hurl syntax v0** — wire up Candidate A (POST + fenced `` ```protobuf `` body) in a Hurl branch, run the sample
   `.hurl` files in `samples/` against the Python server.
7. **`.proto` text parser** — only if descriptor-sets aren't enough.
8. **Server reflection** — optional, once everything else works (and only if we can drive it without true bidi
   streaming).
9. **Compression, well-known types, polish**.

Streaming (server / client / bidi) is **not** on this list. It's a follow-up phase once unary is solid.

---

## 8. Risks and exits

- **Risk: libcurl trailers** — if we can't read HTTP/2 trailers from libcurl, v1 is in trouble. Investigate first,
  before writing any protobuf code.
- **Risk: `.proto` parser scope creep** — mitigated by accepting descriptor sets as the primary input.
- **Exit ramp** — if any of 6.1–6.4 turns out to need substantially more code than expected, we revisit the
  no-third-party-crates constraint with concrete numbers (lines of code, maintenance cost) rather than as a principle.
