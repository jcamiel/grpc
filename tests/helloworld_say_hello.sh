#!/usr/bin/env bash
cd "$(dirname "$0")/.."

grpcurl -plaintext -protoset proto/helloworld.protoset -d @ localhost:50051 helloworld.Greeter/SayHello <<'EOF'
{
  "name": "World"
}
EOF
