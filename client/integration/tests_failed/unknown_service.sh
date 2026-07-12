#!/usr/bin/env bash

client \
  --data '{"name":"bob"}' \
  --protoset ../../proto/echo.protoset \
  http://localhost:50051/helloworld.Greeter/SayHello
