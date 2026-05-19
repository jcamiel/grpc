# Python reference gRPC server

A small Python gRPC server used as the target for the Hurl gRPC POC and for any third-party gRPC client we want to
compare during the survey phase (grpcurl, evans, Postman, Insomnia, `buf curl`, plain `curl --http2`, etc.).

The server is intentionally minimal and **unary-only** — see [`../PLAN.md`](../PLAN.md) for the scope rationale.

## Services exposed

| Service              | Method     | Purpose                                                                                                                            |
|----------------------|------------|------------------------------------------------------------------------------------------------------------------------------------|
| `helloworld.Greeter` | `SayHello` | Trivial baseline — `{name} → "Hello, {name}"`.                                                                                     |
| `echo.Echo`          | `Echo`     | Echoes back a "kitchen-sink" `Payload` that exercises every protobuf wire type, plus `received_at: Timestamp`.                     |
| `status.Status`      | `Fail`     | Returns the `grpc-status` code requested in the body, with the supplied `grpc-message`. `code=0` returns `FailReply` successfully. |

[gRPC server reflection](https://github.com/grpc/grpc/blob/master/doc/server-reflection.md) is enabled, so clients that
support it (grpcurl, Postman, etc.) can introspect the schema without a local `.proto` file.

Listens on `[::]:50051` (plaintext, no TLS). Override with `GRPC_PORT=<n>`.

## First-time setup

All commands are run from the **repo root** (`/Users/jc/Documents/Dev/grpc`), not from inside `server/`.

### 1. Create the virtual environment

```sh
python3.12 -m venv .venv
```

Python 3.12 is what this POC was built and tested against. 3.11–3.13 should also work; 3.14 wheels for `grpcio` may
not be available yet at the time of writing.

The `.venv/` directory is in `.gitignore`.

### 2. Install dependencies

Pick one:

```sh
# Loose, latest-compatible versions (good for fresh work):
.venv/bin/pip install -r server/requirements.txt

# Exact versions known to work (good for reproducibility):
.venv/bin/pip install -r server/requirements-frozen.txt
```

`requirements.txt` holds the three direct dependencies (`grpcio`, `grpcio-tools`, `grpcio-reflection`).
`requirements-frozen.txt` is the output of `pip freeze --local` from a working install and pins every transitive
package.

### 3. Generate the Python stubs from the `.proto` files

```sh
.venv/bin/python -m grpc_tools.protoc \
  -Iproto \
  --python_out=server/_generated \
  --grpc_python_out=server/_generated \
  proto/helloworld.proto proto/echo.proto proto/status.proto
```

This writes `*_pb2.py` and `*_pb2_grpc.py` under `server/_generated/`. Re-run it any time the `.proto` files change.

## Running the server

```sh
.venv/bin/python -m server
```

Expected output:

```
gRPC server listening on [::]:50051
services:
  - helloworld.Greeter
  - echo.Echo
  - status.Status
  - grpc.reflection.v1alpha.ServerReflection
```

Stop with `Ctrl-C` (handled cleanly).

To use a different port:

```sh
GRPC_PORT=50052 .venv/bin/python -m server
```

## Quick check with `grpcurl`

If you have [`grpcurl`](https://github.com/fullstorydev/grpcurl) installed, all three services work via reflection —
no local `.proto` needed:

```sh
grpcurl -plaintext localhost:50051 list
grpcurl -plaintext -d '{"name": "Hurl"}' localhost:50051 helloworld.Greeter/SayHello
grpcurl -plaintext -d '{"code": 5, "message": "nope"}' localhost:50051 status.Status/Fail
```

## Re-freezing dependencies

After updating versions in `requirements.txt`:

```sh
.venv/bin/pip install --upgrade -r server/requirements.txt
.venv/bin/pip freeze --local > server/requirements-frozen.txt
```
