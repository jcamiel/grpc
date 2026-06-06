#!/usr/bin/env bash
cd "$(dirname "$0")/.."

grpcurl -plaintext -protoset proto/status.protoset -d @ localhost:50051 status.Status/Fail <<'EOF'
{
  "code": 0,
  "message": "all good"
}
EOF
