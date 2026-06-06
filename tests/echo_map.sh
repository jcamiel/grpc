#!/usr/bin/env bash
cd "$(dirname "$0")/.."

grpcurl -plaintext -protoset proto/echo.protoset -d @ localhost:50051 echo.Echo/Echo <<'EOF'
{
  "payload": {
    "tags": {
      "a": 1,
      "b": 2,
      "c": 3
    }
  }
}
EOF
