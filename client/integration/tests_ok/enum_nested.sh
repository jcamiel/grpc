#!/usr/bin/env bash

client \
  --data '{"payload":{"priority":"MEDIUM"}}' \
  --protoset ../proto/echo.protoset \
  http://localhost:50051/echo.Echo/Echo
