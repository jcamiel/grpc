#!/usr/bin/env bash

client \
  --data '{"payload":{"text":"boo👻","i32":-1238765,"s32":-1238765,"flag":true}}' \
  --protoset ../proto/echo.protoset \
  http://localhost:50051/echo.Echo/Echo
