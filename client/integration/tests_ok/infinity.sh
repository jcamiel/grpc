#!/usr/bin/env bash

client \
  --data '{"payload":{"fl":1e39}}' \
  --protoset ../proto/echo.protoset \
  http://localhost:50051/echo.Echo/Echo
