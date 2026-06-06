#!/usr/bin/env bash
cd "$(dirname "$0")/.."

grpcurl -plaintext -emit-defaults -protoset proto/echo.protoset -d @ localhost:50051 echo.Echo/Echo <<'EOF'
{
  "payload": {
    "text": "explicit empty",
    "nickname": ""
  }
}
EOF
