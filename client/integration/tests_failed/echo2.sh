#!/usr/bin/env bash

client \
  --data '{"payload":{"text":10}}' \
  --protoset ../proto/echo.protoset \
  http://localhost:50051/echo.Echo/Echo