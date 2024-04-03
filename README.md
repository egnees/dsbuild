# DSBuild

## Installation
Before using framework, one should install the following:
  - [Rust and Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
  - [gRPC Protocol Buffer Compiler](https://grpc.io/docs/protoc-installation/)

If all required components are correctly installed, one must be able to build project from the root of repository without errors:
```
cargo build
```

## Examples
See [examples submodule](https://egnees.github.io/dsbuild/docs/dsbuild/examples/index.html) and [bin subfolder](https://github.com/egnees/dsbuild/tree/master/bin).

## Running examples

To run [ping process example](https://github.com/egnees/dsbuild/blob/master/bin/pinger.rs), type from the root of repository:
```
cargo run --bin pinger <listen_host> <listen_port> <ponger_host> <ponger_port>
```

To run [pong process example](https://github.com/egnees/dsbuild/blob/master/bin/ponger.rs), type from the root of repository:
```
cargo run --bin ponger <listen_host> <listen_port>
```

After this `pinger` and `ponger` processes must start communication between each other.

To run [ping-pong real example](https://egnees.github.io/dsbuild/docs/dsbuild/examples/ping_pong/real/index.html), which launches `pinger` and `ponger` in different threads,
type from the root of repository:
```
cargo run --bin ping-pong-real
```

To run [testing ping-pong in simulation example](https://egnees.github.io/dsbuild/docs/dsbuild/examples/ping_pong/sim/index.html), type from the root of repository:
```
cargo run --bin ping-pong-sim
```

## Documentation
Available [here](https://egnees.github.io/dsbuild/docs/dsbuild/).

