#!/usr/bin/env bash

client \
  --data '{"name":"bob"}' \
  --protoset ../proto/helloworld.protoset \
  http://localhost:50051/helloworld
