#!/usr/bin/env bash

client \
  --data '{"name":18}' \
  --protoset ../proto/helloworld.protoset \
  http://localhost:50051/helloworld.Greeter/SayHello