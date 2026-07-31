#!/usr/bin/env bash

client \
  --data '{"payload":{"text":"boo👻","i32":-1238765,"s32":-1238765,"flag":true,"u32":43426,"f32":321,"sf64":-45634,"i64":-264836,"s64":-264836,"u64":617,"f64":12345,"d":3.141592653589793,"fl":0.1,"blob":"YWJjMTIzIT8kKiYoKSctPUB+","color":"RED"}}' \
  --protoset ../proto/echo.protoset \
  http://localhost:50051/echo.Echo/Echo
