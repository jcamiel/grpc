#!/usr/bin/env bash

client \
  --data '{"payload":{"sf64":9223372036854775808}}' \
  --protoset ../proto/echo.protoset \
  http://localhost:50051/echo.Echo/Echo