#!/usr/bin/env bash
cd "$(dirname "$0")/.."

# Same call as operation_multiply.sh, but with a SECOND -protoset that
# ships google.rpc.ErrorInfo (and the rest of google.rpc.error_details).
# This lets grpcurl fully decode the payload inside the Any that lives in
# the `grpc-status-details-bin` trailer. Diff the two outputs side-by-side:
# without the extra protoset you get @type + base64 @value; with it you get
# the ErrorInfo fields (`reason`, `domain`, `metadata`).
grpcurl -plaintext \
  -protoset proto/operation.protoset \
  -protoset proto/error_details.protoset \
  -d @ localhost:50051 operation.OperationService/Compute <<'EOF'
{
  "operation": "MULTIPLY",
  "operands": ["2", "3"]
}
EOF
