#!/usr/bin/env bash

client \
  --data '{"payload":{"d":42}}' \
  --protoset ../proto/echo.protoset \
  http://localhost:50051/echo.Echo/Echo
