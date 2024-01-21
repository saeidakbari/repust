# Repust

Repust is a Proxy for Redis and Memcached with Active-Active Replication and Multi Region Support. it is built for adding High Availability and Multi Region Support to Redis and Memcached. since it is a proxy, it can be used with any client that supports Redis or Memcached protocols. Repust is heavily inspired by [twemproxy](https://github.com/twitter/twemproxy) and [dynomite](https://github.com/Netflix/dynomite).

## Features

+ Fast.
+ Lightweight (Written in Rust with performance in mind).
+ Support proxying to single or multiple Redis and Memcached instances.
+ Compatible with Redis and Memcached clients.
+ Support Redis Cluster. (Redis only)
+ Easy to configure. (simple YAML configuration file)
+ Observability (Metrics and Traces).
+ Using consistent hashing for sharding.
+ Active-Active and Multi-Region Replication. (In Progress)

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## License

Repust is licensed under the [MIT License](LICENSE).
