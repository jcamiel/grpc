#!/usr/bin/env bash

client \
  --data '{"payload":{"i64":"1152921504606846976"}}' \
  --protoset ../proto/echo.protoset \
  http://localhost:50051/echo.Echo/Echo
