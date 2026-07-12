#!/usr/bin/env bash

client \
  --data '{"payload":{"text":"boo👻","i32":-2147483649}}' \
  --protoset ../../proto/echo.protoset \
  http://localhost:50051/echo.Echo/Echo
