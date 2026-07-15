#!/usr/bin/env bash

client \
  --data '{"payload":{"name":"foo"}}' \
  --protoset ../proto/echo.protoset \
  http://localhost:50051/echo.Echo/Echo