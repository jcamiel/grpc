"""Python reference gRPC server for the Hurl POC.

Three unary services, picked to cover the wire-format cases the Rust POC
needs to handle:

  - helloworld.Greeter.SayHello — trivial baseline.
  - echo.Echo.Echo              — kitchen-sink message (every wire type).
  - status.Status.Fail          — returns arbitrary grpc-status codes for
                                  client-side error-handling tests.

Server reflection is enabled so reflection-driven clients (grpcurl, evans,
Postman, buf curl) can introspect the schema without a local .proto file.
"""

from __future__ import annotations

import os
import signal
import sys
from concurrent import futures
from pathlib import Path

# The generated *_pb2.py files import each other by their unqualified module
# names (`import echo_pb2 as ...`), which only resolves if the directory
# containing them is on sys.path. Insert it before any of those imports run.
_GEN = Path(__file__).parent / "_generated"
sys.path.insert(0, str(_GEN))

import grpc
from google.protobuf import timestamp_pb2
from grpc_reflection.v1alpha import reflection

import echo_pb2
import echo_pb2_grpc
import helloworld_pb2
import helloworld_pb2_grpc
import status_pb2
import status_pb2_grpc


DEFAULT_PORT = 50051


class GreeterServicer(helloworld_pb2_grpc.GreeterServicer):
    def SayHello(self, request, context):
        name = request.name or "world"
        return helloworld_pb2.HelloReply(message=f"Hello, {name}")


class EchoServicer(echo_pb2_grpc.EchoServicer):
    def Echo(self, request, context):
        received_at = timestamp_pb2.Timestamp()
        received_at.GetCurrentTime()
        return echo_pb2.EchoReply(payload=request.payload, received_at=received_at)


# Build int → grpc.StatusCode once. grpc.StatusCode members carry a
# (numeric, name) tuple in `.value`.
_STATUS_BY_CODE = {sc.value[0]: sc for sc in grpc.StatusCode}


class StatusServicer(status_pb2_grpc.StatusServicer):
    def Fail(self, request, context):
        if request.code == 0:
            return status_pb2.FailReply(note=request.message or "ok")
        code = _STATUS_BY_CODE.get(request.code, grpc.StatusCode.UNKNOWN)
        context.set_code(code)
        context.set_details(request.message or f"intentional failure: {code.name}")
        return status_pb2.FailReply()


def _service_full_names() -> tuple[str, ...]:
    return (
        helloworld_pb2.DESCRIPTOR.services_by_name["Greeter"].full_name,
        echo_pb2.DESCRIPTOR.services_by_name["Echo"].full_name,
        status_pb2.DESCRIPTOR.services_by_name["Status"].full_name,
        reflection.SERVICE_NAME,
    )


def serve() -> None:
    port = int(os.environ.get("GRPC_PORT", DEFAULT_PORT))
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))

    helloworld_pb2_grpc.add_GreeterServicer_to_server(GreeterServicer(), server)
    echo_pb2_grpc.add_EchoServicer_to_server(EchoServicer(), server)
    status_pb2_grpc.add_StatusServicer_to_server(StatusServicer(), server)
    reflection.enable_server_reflection(_service_full_names(), server)

    address = f"[::]:{port}"
    server.add_insecure_port(address)
    server.start()
    print(f"gRPC server listening on {address}", flush=True)
    print("services:", flush=True)
    for name in _service_full_names():
        print(f"  - {name}", flush=True)

    def _shutdown(signum, _frame):
        print(f"\nreceived signal {signum}, shutting down...", flush=True)
        server.stop(grace=1).wait()
        sys.exit(0)

    signal.signal(signal.SIGINT, _shutdown)
    signal.signal(signal.SIGTERM, _shutdown)

    server.wait_for_termination()
