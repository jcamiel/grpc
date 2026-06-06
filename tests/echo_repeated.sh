#!/usr/bin/env bash
cd "$(dirname "$0")/.."

grpcurl -plaintext -protoset proto/echo.protoset -d @ localhost:50051 echo.Echo/Echo <<'EOF'
{
  "payload": {
    "packed_ints": [1, 2, 3, 100, -1],
    "strings": ["alpha", "beta", "gamma"]
  }
}
EOF
