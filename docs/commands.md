# Redis Command Support

Redis proxies act as intermediaries between clients and Redis servers. Their primary function is to efficiently route
commands and responses. However, some Redis commands are not suitable for proxying due to their characteristics.

These unsupported commands often involve long-running operations or require persistent connections.
For instance, migrating data between servers (MIGRATE) or waiting for specific events (BLPOP) necessitates a long-lived
connection that a proxy cannot guarantee. Additionally, some commands (like PUBLISH/SUBSCRIBE) establish stateful
connections for real-time data updates. Proxies, designed for stateless request-response interactions, wouldn't be able
to manage the state changes and maintain context across multiple requests.

In simpler terms, proxies work best for quick exchanges of data. They can't handle commands that require them to act
like servers themselves, hold onto information between requests, or wait for extended periods.

It is important to note that Repust adheres to the RESP2 protocol standard. If a required Redis command is not
listed as supported, there are two ways to address this. You can either open an issue detailing the missing
functionality, or submit a pull request to directly add support for the desired command.

## Supported Commands

This table summarizes the Redis commands supported by Repust, categorized by their function in Redis and indicates
whether they are read or write operations. also includes an example usage pattern and the time complexity of the command.

**Note:** Time complexity is a theoretical measure of how the execution time of a command grows with the size of the input data.

### Key Operations

| Command | Supported | Type | Example | Complexity |
|---|---|---|---|---|
| DEL| Yes | Write | `DEL mykey` | O(1) |
| UNLINK | Yes | Write | `UNLINK expiredkey` | O(1) per key |
| DUMP | No | N/A | N/A | N/A |
| EXISTS | Yes | Read | `EXISTS keyname` | O(1) |
| EXPIRE | Yes | Write | `EXPIRE product_cache 3600` | O(1) |
| EXPIREAT | Yes | Write | `EXPIREAT session_data 1655200000` | O(1) |
| KEYS | Yes | Read | `KEYS user-*` | O(N) (N = number of matching keys) |
| DBSIZE | Yes | Read | `DBSIZE` | O(1) |
| MIGRATE | No | N/A | N/A | N/A |
| MOVE | No | N/A | N/A | N/A |
| OBJECT | No | N/A | N/A | N/A |
| PERSIST | Yes | Write | `PERSIST shopping_cart` | O(1) |
| PEXPIRE | Yes | Write | `PEXPIRE session_id 1800000` | O(1) |
| PEXPIREAT | Yes | Write | `PEXPIREAT login_attempt 1655200100` | O(1) |
| PTTL | Yes | Read | `PTTL access_token` | O(1) |
| RANDOMKEY | No | N/A | N/A | N/A |
| RENAME | No | N/A | N/A | N/A |
| RENAMENX | No | N/A | N/A | N/A |
| RESTORE | Yes | Write | `RESTORE counter 3600 "i:10"` | O(N) (N = data size) |
| SCAN | Yes | Read | `SCAN 0 user* 10` | O(M) (M = number of matching keys) |
| SORT | Yes | Write | `SORT products BY price LIMIT 0 10 ASC` | O(N * log(N)) (average) |
| TTL | Yes | Read | `TTL product_info` | O(1) |
| TYPE | Yes | Read | `TYPE mylist` | O(1) |
| WAIT | No | N/A | N/A | N/A |
| COMMAND | Yes | Read | `COMMAND` | O(1) |
| CLIENT | Yes | Read | `CLIENT LIST` | O(N) (N = number of clients) |
| MODULE | Yes | Read | `MODULE LIST` | O(1) |
| MEMORY | Yes | Read | `MEMORY USAGE` | O(1) |

### String Operations

| Command | Supported | Type | Example | Complexity |
|---|---|---|---|---|
| APPEND key value | Yes | Write | `APPEND message Hello world!` | O(1) (amortized) |
| BITCOUNT | Yes | Read | `BITCOUNT mycounter` | O(N) (N = number of bits set) |
| BITOP | No | N/A | N/A | N/A |
| BITPOSbit  | Yes | Read | `BITPOS flags 1` | O(N) (N = number of bits) |
| APPEND | Yes | Write | `APPEND mykey "value"` | O(1) |
| BITCOUNT | Yes | Read | `BITCOUNT mykey [start] [end]` | O(N) |
| BITOP | No | N/A | N/A | N/A |
| BITPOS | Yes | Read | `BITPOS mykey bit [start] [end]` | O(N) |
| DECR | Yes | Write | `DECR mykey` | O(1) |
| DECRBY | Yes | Write | `DECRBY mykey decrement` | O(1) |
| GET | Yes | Read | `GET mykey` | O(1) |
| GETBIT | Yes | Read | `GETBIT mykey offset` | O(1) |
| GETRANGE | Yes | Read | `GETRANGE mykey start end` | O(N) |
| GETSET | Yes | Write | `GETSET mykey "newvalue"` | O(1) |
| INCR | Yes | Write | `INCR mykey` | O(1) |
| INCRBY | Yes | Write | `INCRBY mykey increment` | O(1) |
| INCRBYFLOAT | Yes | Write | `INCRBYFLOAT mykey increment` | O(1) |
| MGET | Yes | Read | `MGET key1 key2 ... keyN` | O(N) |
| MSET | Yes | Write | `MSET key1 value1 key2 value2 ... keyN valueN` | O(N) |
| MSETNX | No | N/A | N/A | N/A |
| PSETEX | Yes | Write | `PSETEX mykey milliseconds value` | O(1) |
| SET | Yes | Write | `SET mykey "value"` | O(1) |
| SETBIT | Yes | Write | `SETBIT mykey offset value` | O(1) |
| SETEX | Yes | Write | `SETEX mykey seconds value` | O(1) |
| SETNX | Yes | Write | `SETNX mykey "value"` | O(1) |
| SETRANGE | Yes | Write | `SETRANGE mykey offset value` | O(1) |
| BITFIELD | Yes | Write | `BITFIELD mykey [GET type offset] [SET type offset value] [INCRBY type offset increment] [OVERFLOW wrap/sat/fail]` | O(1) |
| STRLEN | Yes | Read | `STRLEN mykey` | O(1) |
| SUBSTR | Yes | Read | `SUBSTR mykey start end` | O(N) |

### Hash Operations

| Command | Supported | Type | Example | Complexity |
|---|---|---|---|---|
| HDEL | Yes | Write | `HDEL myhash field1 field2 ... fieldN` | O(N) |
| HEXISTS | Yes | Read | `HEXISTS myhash field` | O(1) |
| HGET | Yes | Read | `HGET myhash field` | O(1) |
| HGETALL | Yes | Read | `HGETALL myhash` | O(N) |
| HINCRBY | Yes | Write | `HINCRBY myhash field increment` | O(1) |
| HINCRBYFLOAT | Yes | Write | `HINCRBYFLOAT myhash field increment` | O(1) |
| HKEYS | Yes | Read | `HKEYS myhash` | O(N) |
| HLEN | Yes | Read | `HLEN myhash` | O(1) |
| HMGET | Yes | Read | `HMGET myhash field1 field2 ... fieldN` | O(N) |
| HMSET | Yes | Write | `HMSET myhash field1 value1 field2 value2 ... fieldN valueN` | O(N) |
| HSET | Yes | Write | `HSET myhash field value` | O(1) |
| HSETNX | Yes | Write | `HSETNX myhash field value` | O(1) |
| HSTRLEN | Yes | Read | `HSTRLEN myhash field` | O(1) |
| HVALS | Yes | Read | `HVALS myhash` | O(N) |
| HSCAN | Yes | Read | `HSCAN myhash cursor [MATCH pattern] [COUNT count]` | O(N) |

### List Operations

| Command | Supported | Type | Example | Complexity |
|---|---|---|---|---|
| BLPOP | No | N/A | N/A | N/A |
| BRPOP | No | N/A | N/A | N/A |
| BRPOPLPUSH | No | N/A | N/A | N/A |
| LINDEX | Yes | Read | `LINDEX mylist index` | O(N) |
| LINSERT | Yes | Write | `LINSERT mylist BEFORE/AFTER pivot value` | O(N) |
| LLEN | Yes | Read | `LLEN mylist` | O(1) |
| LPOP | Yes | Write | `LPOP mylist` | O(1) |
| LPUSH | Yes | Write | `LPUSH mylist value1 value2 ... valueN` | O(1) |
| LPUSHX | Yes | Write | `LPUSHX mylist value` | O(1) |
| LRANGE | Yes | Read | `LRANGE mylist start stop` | O(N) |
| LREM | Yes | Write | `LREM mylist count value` | O(N) |
| LSET | Yes | Write | `LSET mylist index value` | O(N) |
| LTRIM | Yes | Write | `LTRIM mylist start stop` | O(N) |
| RPOP | Yes | Write | `RPOP mylist` | O(1) |
| RPOPLPUSH | Yes | Write | `RPOPLPUSH source destination` | O(1) |
| RPUSH | Yes | Write | `RPUSH mylist value1 value2 ... valueN` | O(1) |
| RPUSHX | Yes | Write | `RPUSHX mylist value` | O(1) |

### Set Operations

| Command | Supported | Type | Example | Complexity |
|---|---|---|---|---|
| SADD | Yes | Write | `SADD myset member1 member2 ... memberN` | O(1) |
| SCARD | Yes | Read | `SCARD myset` | O(1) |
| SDIFF | Yes | Read | `SDIFF key1 key2 ... keyN` | O(N) |
| SDIFFSTORE | Yes | Write | `SDIFFSTORE destination key1 key2 ... keyN` | O(N) |
| SINTER | Yes | Read | `SINTER key1 key2 ... keyN` | O(N) |
| SINTERSTORE | Yes | Write | `SINTERSTORE destination key1 key2 ... keyN` | O(N) |
| SISMEMBER | Yes | Read | `SISMEMBER myset member` | O(1) |
