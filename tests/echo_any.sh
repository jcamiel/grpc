#!/usr/bin/env bash
cd "$(dirname "$0")/.."

grpcurl -plaintext -protoset proto/echo.protoset -d @ localhost:50051 echo.Echo/Echo <<'EOF'
{
  "payload": {
    "any_field": {
      "@type": "type.googleapis.com/echo.Nested",
      "label": "wrapped",
      "value": 42
    }
  }
}
EOF
