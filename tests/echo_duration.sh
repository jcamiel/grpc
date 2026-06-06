#!/usr/bin/env bash
cd "$(dirname "$0")/.."

grpcurl -plaintext -protoset proto/echo.protoset -d @ localhost:50051 echo.Echo/Echo <<'EOF'
{
  "payload": {
    "duration": "1.5s"
  }
}
EOF
