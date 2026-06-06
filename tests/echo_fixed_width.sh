#!/usr/bin/env bash
cd "$(dirname "$0")/.."

grpcurl -plaintext -protoset proto/echo.protoset -d @ localhost:50051 echo.Echo/Echo <<'EOF'
{
  "payload": {
    "f64": "18446744073709551615",
    "sf64": "-9000000000000",
    "d": 3.141592653589793,
    "f32": 4294967295,
    "sf32": -2000000000,
    "fl": 2.71828
  }
}
EOF
