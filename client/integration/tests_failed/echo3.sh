#!/usr/bin/env bash

client \
  --data '{"payload":{"text":"bob","u32":-1}}' \
  --protoset ../proto/echo.protoset \
  http://localhost:50051/echo.Echo/Echo