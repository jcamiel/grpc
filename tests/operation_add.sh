#!/usr/bin/env bash
cd "$(dirname "$0")/.."

grpcurl -plaintext -protoset proto/operation.protoset -d @ localhost:50051 operation.OperationService/Compute <<'EOF'
{
  "operation": "ADD",
  "operands": ["1", "2", "3", "4"]
}
EOF
