# gRPC Clients Survey

## grpcurl

Reflection:

```shell
$ grpcurl -plaintext localhost:50051 list
echo.Echo
grpc.reflection.v1alpha.ServerReflection
helloworld.Greeter
status.Status
```

```shell
$ grpcurl -plaintext -d '{"name": "Bob"}' localhost:50051 helloworld.Greeter/SayHello
{
  "message": "Hello, Bob"
}
```

Response on stdout is JSON.

With verbosity:

```shell
$ grpcurl -v -plaintext -d '{"name": "Bob"}' localhost:50051 helloworld.Greeter/SayHello

Resolved method descriptor:
rpc SayHello ( .helloworld.HelloRequest ) returns ( .helloworld.HelloReply );

Request metadata to send:
(empty)

Response headers received:
content-type: application/grpc
grpc-accept-encoding: identity, deflate, gzip

Response contents:
{
  "message": "Hello, Bob"
}

Response trailers received:
(empty)
Sent 1 request and received 1 response
```


## buf curl

```shell
$ buf curl --data '{"name": "Bob"}' http://localhost:50051/helloworld.Greeter/SayHello
{
  "message": "Hello, Bob"
}
```

With verbosity:

```shell
$ buf curl --debug --protocol grpc --http2-prior-knowledge --data '{"name": "Bob"}' http://localhost:50051/helloworld.Greeter/SayHello
DEBUG	github.com/bufbuild/buf/private/buf/bufcurl.NewWKTResolver	{"duration":"6.708µs"}
DEBUG	github.com/bufbuild/buf/private/bufpkg/bufimage.BuildImage	{"duration":"13.549875ms"}
{
  "message": "Hello, Bob"
}
```

