#!/usr/bin/env bash

client \
  --data '{"payload":{"sf32":-1234,"text":"foo"}}' \
  --protoset ../proto/echo.protoset \
  http://localhost:50051/echo.Echo/Echo
