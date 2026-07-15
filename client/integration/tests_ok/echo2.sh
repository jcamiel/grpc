#!/usr/bin/env bash

client \
  --data '{"payload":{"text":"foo", "sf32":-1234}}' \
  --protoset ../proto/echo.protoset \
  http://localhost:50051/echo.Echo/Echo