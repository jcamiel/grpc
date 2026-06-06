#!/usr/bin/env bash
cd "$(dirname "$0")/.."

grpcurl -plaintext -protoset proto/echo.protoset -d @ localhost:50051 echo.Echo/Echo <<'EOF'
{
  "payload": {
    "i32": -7,
    "i64": "-99999999999",
    "u32": 42,
    "u64": "999999999999",
    "s32": -1,
    "s64": "-1",
    "flag": true
  }
}
EOF
