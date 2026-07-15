#!/usr/bin/env bash

client \
  --data '{"payload":{"text":"boo👻","i32":-1238765,"s32":-1238765,"flag":true,"u32":43426,"f32":321}}' \
  --protoset ../proto/echo.protoset \
  http://localhost:50051/echo.Echo/Echo
