#!/usr/bin/env bash

client \
  --data '{"foo":"bar"}' \
  --protoset ../../proto/helloworld.protoset \
  http://localhost:50051/helloworld.Greeter/SayHello