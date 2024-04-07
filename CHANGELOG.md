# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org).

<!--
Note: In this file, do not use the hard wrap in the middle of a sentence for compatibility with GitHub comment style markdown rendering.
-->

## [Unreleased]

## [0.1.1-PRERELEASE] - 2024-04-07

### Added

+ Supporting Apple-Darwin and PC-Windows release targets

## [0.1.0] - 2024-04-07

### Added

+ Supporting Redis and Memcached protocols.
+ Implementing consistent hashing for key distribution using Ketama algorithm.
+ Connecting to multiple backend servers with user-defined names (aliasing).
+ Weights traffic across backend servers for optimized load balancing.
+ Including comprehensive logging for request tracking and debugging.
+ Integrating OpenTelemetry for detailed performance monitoring.
+ Utilize RESP2 serialization for efficient data exchange.
+ Employing asynchronous operations with Tokio and Futures for non-blocking functionality.

[Unreleased]: https://github.com/saeidakbari/repust/compare/v0.1.0...HEAD
[0.1.1-PRERELEASE]: https://github.com/saeidakbari/repust/compare/v0.1.0...v0.1.1
[0.1.0]: https://https://github.com/saeidakbari/repust/tags/v0.1.0
