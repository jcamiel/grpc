# Python reference gRPC server

A small Python gRPC server used as the target for the Hurl gRPC POC and for any third-party gRPC client we want to
compare during the survey phase (grpcurl, evans, Postman, Insomnia, `buf curl`, plain `curl --http2`, etc.).

The server is intentionally minimal and **unary-only** — see [`../PLAN.md`](../PLAN.md) for the scope rationale.

## Services exposed

| Service              | Method     | Purpose                                                                                                                            |
|----------------------|------------|------------------------------------------------------------------------------------------------------------------------------------|
| `helloworld.Greeter`         | `SayHello` | Trivial baseline — `{name} → "Hello, {name}"`.                                                                                     |
| `echo.Echo`                  | `Echo`     | Echoes back a "kitchen-sink" `Payload` that exercises every protobuf wire type, plus `received_at: Timestamp`.                     |
| `status.Status`              | `Fail`     | Returns the `grpc-status` code requested in the body, with the supplied `grpc-message`. `code=0` returns `FailReply` successfully. |
| `operation.OperationService` | `Compute`  | Tiny math service. `ADD` returns the sum of `operands`; `MULTIPLY` returns `UNIMPLEMENTED` with a `google.rpc.Status` + `ErrorInfo` in the `grpc-status-details-bin` trailer (rich error model). |

[gRPC server reflection](https://github.com/grpc/grpc/blob/master/doc/server-reflection.md) is enabled, so clients that
support it (grpcurl, Postman, etc.) can introspect the schema without a local `.proto` file.

Listens on `[::]:50051` (plaintext, no TLS). Override with `GRPC_PORT=<n>`.

## First-time setup

All commands are run from the **repo root** (`/Users/jc/Documents/Dev/grpc`), not from inside `server/`.

### 1. Create the virtual environment

```shell
$ python3.12 -m venv .venv
```

Python 3.12 is what this POC was built and tested against. 3.11–3.13 should also work; 3.14 wheels for `grpcio` may
not be available yet at the time of writing.

The `.venv/` directory is in `.gitignore`.

### 2. Install dependencies

```shell
$ pip install -r server/requirements-frozen.txt
```

`requirements.txt` holds the three direct dependencies (`grpcio`, `grpcio-tools`, `grpcio-reflection`).
`requirements-frozen.txt` is the output of `pip freeze --local` from a working install and pins every transitive
package.

### 3. Generate the Python stubs from the `.proto` files

```shell
$ python -m grpc_tools.protoc \
  -Iproto \
  --python_out=server/_generated \
  --grpc_python_out=server/_generated \
  proto/helloworld.proto proto/echo.proto proto/status.proto
```

This writes `*_pb2.py` and `*_pb2_grpc.py` under `server/_generated/`. Re-run it any time the `.proto` files change.

### 4. Generate the `.protoset` files (descriptor sets)

A `.protoset` is the binary output of `protoc --descriptor_set_out=...` — a serialized `FileDescriptorSet`. It's the
schema source Hurl will consume (see [`../PLAN.md`](../PLAN.md) §6.4). One `.protoset` is generated per service so
that `.hurl` test files can reference just the schema they need:

```shell
$ for p in helloworld echo status operation; do
  .venv/bin/python -m grpc_tools.protoc \
    -Iproto \
    --descriptor_set_out=proto/$p.protoset \
    --include_imports \
    proto/$p.proto
done
```

`--include_imports` makes each `.protoset` self-contained — `echo.protoset` therefore bundles its `google.protobuf`
well-known-type dependencies (`Timestamp`, `Duration`, `Any`) inline.

This writes `proto/helloworld.protoset`, `proto/echo.protoset`, `proto/status.protoset`, `proto/operation.protoset`.
Re-run any time the matching `.proto` files change.

There's also a stand-alone `proto/error_details.protoset` for `google.rpc.ErrorInfo` etc., compiled from the
`googleapis-common-protos` package that `grpcio-status` pulls in. It's useful as a *second* `-protoset` alongside
`operation.protoset` when you want grpcurl to decode the payload inside the `grpc-status-details-bin` trailer. Build
it like this:

```shell
$ .venv/bin/python -m grpc_tools.protoc \
  -I .venv/lib/python3.14/site-packages \
  --descriptor_set_out=proto/error_details.protoset \
  --include_imports \
  google/rpc/error_details.proto
```

(Adjust the `python3.14` component to match your `.venv`.)

## Running the server

```shell
$ python -m server
```

Expected output:

```
gRPC server listening on [::]:50051
services:
  - helloworld.Greeter
  - echo.Echo
  - status.Status
  - operation.OperationService
  - grpc.reflection.v1alpha.ServerReflection
```

Stop with `Ctrl-C` (handled cleanly).

To use a different port:

```shell
$ GRPC_PORT=50052 python -m server
```

## Quick check with `grpcurl`

If you have [`grpcurl`](https://github.com/fullstorydev/grpcurl) installed, all three services work via reflection —
no local `.proto` needed:

```shell
$ grpcurl -plaintext localhost:50051 list
$ grpcurl -plaintext -d '{"name": "Bob"}' localhost:50051 helloworld.Greeter/SayHello
$ grpcurl -plaintext -d '{"code": 5, "message": "nope"}' localhost:50051 status.Status/Fail
```

## Re-freezing dependencies

After updating versions in `requirements.txt`:

```shell
$ pip install --upgrade -r server/requirements.txt
$ pip freeze --local > server/requirements-frozen.txt
```
