# grpcurl spot-check scripts

One [`grpcurl`](https://github.com/fullstorydev/grpcurl) call per file. Each script is fully self-contained — no shared
helpers, no sourcing. Pick the case you want and run it. The JSON request body is fed as a strong heredoc (`<<'EOF'`)
so it travels to `grpcurl` exactly as written.

## Prerequisites

1. `grpcurl` on `PATH` (`brew install grpcurl` on macOS).
2. Server running on `localhost:50051`: `.venv/bin/python -m server`.
3. `.protoset` files in `../proto/` — see [`../server/README.md`](../server/README.md) step 4.

Each script `cd`s to the repo root, so it works whether you invoke it from the repo root, from `tests/`, or via an
absolute path.

## Scripts

### `helloworld.Greeter`

| Script                       | What it sends                |
|------------------------------|------------------------------|
| `helloworld_say_hello.sh`    | `SayHello{name: "World"}`    |

### `status.Status`

| Script                       | Code | grpc-status         |
|------------------------------|------|---------------------|
| `status_ok.sh`               | 0    | OK (returns reply)  |
| `status_not_found.sh`        | 5    | NOT_FOUND           |
| `status_unauthenticated.sh`  | 16   | UNAUTHENTICATED     |

### `echo.Echo` — one script per wire-format case

| Script                   | Exercises                                                                           |
|--------------------------|-------------------------------------------------------------------------------------|
| `echo_empty.sh`          | Empty payload                                                                       |
| `echo_varints.sh`        | `int32`, `int64`, `uint32`, `uint64`, `sint32`, `sint64`, `bool` (varint wire type) |
| `echo_fixed_width.sh`    | `fixed32/64`, `sfixed32/64`, `float`, `double` (32-bit and 64-bit wire types)       |
| `echo_string_bytes.sh`   | `string` + `bytes` (base64 in protojson)                                            |
| `echo_nested.sh`         | Nested message                                                                      |
| `echo_repeated.sh`       | `repeated int32` (packed) + `repeated string`                                       |
| `echo_oneof_text.sh`     | `oneof` — `choice_text` branch                                                      |
| `echo_oneof_int.sh`      | `oneof` — `choice_int` branch                                                       |
| `echo_map.sh`            | `map<string, int32>`                                                                |
| `echo_enum_color.sh`     | File-level enum (`Color`)                                                           |
| `echo_enum_priority.sh`  | Message-level / nested enum (`Payload.Priority`)                                    |
| `echo_optional_unset.sh` | proto3 `optional` — field absent (with `-emit-defaults` to make it visible)         |
| `echo_optional_empty.sh` | proto3 `optional` — set to empty string (the case `optional` exists for)            |
| `echo_optional_set.sh`   | proto3 `optional` — set to a value                                                  |
| `echo_duration.sh`       | Well-known type `google.protobuf.Duration`                                          |
| `echo_any.sh`            | Well-known type `google.protobuf.Any` (wraps an `echo.Nested`)                      |

## Overriding the host

Each script hardcodes `localhost:50051`. Edit in place if you need a different host — that's faster than adding plumbing
for a one-line change.

## Why `-protoset` instead of reflection?

The server has reflection enabled, but the scripts pass `-protoset` explicitly because that matches the path Hurl will
take (see [`../PLAN.md`](../PLAN.md) §3 / §4). Drop the `-protoset` argument to compare against the reflection-based
flow.
