# furnace-rs-grpc

Managed protobuf services and named generated clients for Furnace, with native Tonic requests, responses, statuses, and streams. gRPC is available directly; enable `jwt` for Passport protection and `tls` for verified TLS. The `furnace-rs` facade exposes these through `grpc` and `grpc-tls`.

Applications own their `.proto` files and generated client/server code. Detailed usage documentation is completed with the transport feature's example and compatibility verification task.
