#!/usr/bin/env bash

client \
  --data '{"payload":{"text":"","i32":0,"s32":0,"flag":false,"u32":0,"f32":0,"sf64":0}}' \
  --protoset ../proto/echo.protoset \
  http://localhost:50051/echo.Echo/Echo
