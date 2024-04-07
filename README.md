# Repust

Redis/Memcached Proxy in Rust

[![Build status](https://github.com/saeidakbari/repust/actions/workflows/ci.yml/badge.svg?branch=mainn)](https://github.com/saeidakbari/repust/actions/workflows/ci.yml)

![repust logo](.assets/logo.png)

Repust is a Proxy for Redis and Memcached with Active-Active Replication and Multi Region Support. it is built for adding High Availability and Multi Region Support to Redis and Memcached. since it is a proxy, it can be used with any client that supports Redis or Memcached protocols. Repust is heavily inspired by [twemproxy](https://github.com/twitter/twemproxy), [dynomite](https://github.com/Netflix/dynomite)

Some of the Repust capabilities are taken from [RCProxy](https://github.com/clia/rcproxy) but completely
rewritten to be more Rust idiomatic and performant.

## Features

+ Fast.
+ Lightweight (Written in Rust with performance in mind).
+ Support proxying to single or multiple Redis and Memcached instances.
+ Compatible with Redis and Memcached clients.
+ Easy to configure. (simple TOML configuration file)
+ Observability (Metrics and Traces).
+ Using consistent hashing for sharding.
+ Support Redis Cluster. (TODO)
+ Active-Active and Multi-Region Replication. (TODO)

## Warning

NOTE: Repust is still in the early stages of development and should not be considered production-ready.
Although maintainers are trying to keep the API stable and avoid major changes in the future before the 1.0 release,
active development is ongoing and it may cause braking changes. For production environments, use it at your own risk.

## Run

Download the latest release from the [releases page](https://github.com/saeidakbari/repust/releases) and extract it. You
can also build Repust from source. for building Repust, check out the [Development](#development) section.
In order to run Repust, you need to have configuration file in place. you can find an example configuration file in `config.toml.example` file.
After that, you can run Repust with the following command:

```bash
repust -c config.toml
```

For more information about the available options, you can run the following command:

```bash
$ repust --help

Repust Redis/Memcached proxy server

Usage: repust [OPTIONS]

Options:
  -a, --app-name <APP_NAME>
          App name, used for overriding the default app name in telemetry [default: repust]
  -c, --config-file-addr <CONFIG_FILE_ADDR>
          Config file path [default: config.toml]
  -m, --metrics-port <METRICS_PORT>
          Port for exposing metrics [default: 9001]
  -h, --help
          Print help (see more with '--help')
  -V, --version
          Print version
```

## Configuration

Repust supports different backend cache types including Redis (single and cluster) and Memcached (Memcached and Memcached Binary).
For more in-depth information about the configuration file, you can check the `config.toml.example` file.

## Development

Repust is written in Rust. you need to have Rust installed on your machine. you can install Rust by running the following command:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installing Rust, you can clone the repository and build Repust from source. Repust requires some dependencies listed
in the `Cargo.toml` file. For building Repust, run the following command:

```bash
# For building in debug mode
cargo build

# For building in release mode
cargo build --release
```

## Contributing

Contributions are welcome! Feel free to open an issue or submit a pull request if you have any ideas or bug fixes. It is
preferred to choose a feature to implement from the TODO list or open an issue to discuss your ideas before starting to work on it.
Although Repust is written in Rust and natively high performant, it is always good to have performance in mind while implementing new features.
Repust is a free community-driven project and always will be so any kind of help is appreciated.

This project follows Rust coding style and default cargo formatting. it is recommended to use `rustfmt` for formatting the code
before submitting a pull request. you can run the following command to format the code:

```bash
cargo fmt
```

## TODO

+ Use log configurations from the config file.
+ Add Active-Active Replication.
+ Add Redis Cluster Support.

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Code of Conduct

This project and everyone participating in it is governed by the [Code of Conduct](CODE_OF_CONDUCT.md).
By participating, you are expected to uphold this code.

## License

Repust is licensed under the [MIT License](LICENSE).
