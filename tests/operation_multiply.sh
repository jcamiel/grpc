#!/usr/bin/env bash
cd "$(dirname "$0")/.."

grpcurl -plaintext -protoset proto/operation.protoset -d @ localhost:50051 operation.OperationService/Compute <<'EOF'
{
  "operation": "MULTIPLY",
  "operands": ["2", "3"]
}
EOF
