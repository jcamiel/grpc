#!/usr/bin/env bash

client \
  --data '{"payload":{"f32":-123}}' \
  --protoset ../proto/echo.protoset \
  http://localhost:50051/echo.Echo/Echo