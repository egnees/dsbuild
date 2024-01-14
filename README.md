# DSBuild

## Installation
Before using framework, one should install the following:
  - [Rust compiler](https://www.rust-lang.org/tools/install)
  - [gRPC Protocol Buffer Compiler](https://grpc.io/docs/protoc-installation/)

## Examples
See [examples submodule](https://egnees.github.io/dsbuild/docs/dsbuild/examples/index.html) and [bin subfolder](https://github.com/egnees/dsbuild/tree/master/bin).

## Running examples

To run [PingProcess](https://egnees.github.io/dsbuild/docs/dsbuild/process_lib/ping/struct.PingProcess.html), type from the root of repository
```
cargo run --bin pinger <listen_host> <listen_port> <ponger_host> <ponger_port>
```

To run [PongProcess](https://egnees.github.io/dsbuild/docs/dsbuild/process_lib/pong/struct.PongProcess.html), type from the root of repository
```
cargo run --bin pong <listen_host> <listen_port>
```

After this `pinger` and `ponger` processes must start communication between each other.

To run [ping-pong real example](https://egnees.github.io/dsbuild/docs/dsbuild/examples/ping_pong/real/index.html), which launches `pinger` and `ponger` in different threads,
type from the root of repository
```
cargo run --bin ping-pong-real
```

To run [testing of ping-pong ecosystem in simulation](https://egnees.github.io/dsbuild/docs/dsbuild/examples/ping_pong/sim/index.html), type from the root of repository
```
cargo run --bin ping-pong-sim
```

## Documentation
Available [here](https://egnees.github.io/dsbuild/docs/dsbuild/).

