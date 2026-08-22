# Redis Client for no_std
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Crates.io](https://img.shields.io/crates/v/embedded-redis.svg)](https://crates.io/crates/embedded-redis)
[![Actions Status](https://github.com/pegasus-aero/rt-embedded-redis/workflows/QA/badge.svg)](http://github.com/pegasus-aero/rt-embedded-redis/actions)

This crate offers an asynchronous Redis client for `no_std` targets based on
[`embedded-nal-async`](https://docs.rs/embedded-nal-async/).
Both RESP2 and RESP3 protocol are supported.

## Documentation:
* [Connection management](https://docs.rs/embedded-redis/latest/embedded_redis/network/index.html#connection-handling)
* [Asynchronous response handling](https://docs.rs/embedded-redis/latest/embedded_redis/network/index.html#asynchronous-response-management)
* Popular command examples:
  * [SET command](https://docs.rs/embedded-redis/latest/embedded_redis/commands/set/index.html)
  * [GET command](https://docs.rs/embedded-redis/latest/embedded_redis/commands/get/index.html)
  * [PUBLISH command](https://docs.rs/embedded-redis/latest/embedded_redis/commands/publish/index.html)
* [Command abstraction](https://docs.rs/embedded-redis/latest/embedded_redis/commands/index.html)
* [Custom commands](https://docs.rs/embedded-redis/latest/embedded_redis/commands/custom/index.html)
* [Subscriptions](https://docs.rs/embedded-redis/latest/embedded_redis/subscribe)

## Example
```rust
async fn example() {
use core::str::FromStr;
use core::net::SocketAddr;
use std_embedded_nal_async::Stack;
use std_embedded_time::StandardClock;
use embedded_redis::network::ConnectionHandler;

let stack = Stack::default();
let clock = StandardClock::default();

let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
let mut connection_handler = ConnectionHandler::resp2(server_address);
let client = connection_handler.connect(&stack, Some(&clock)).await.unwrap();

let response = client.set("key", "value").await.unwrap();
}
```

## Development

Any form of support is greatly appreciated. Feel free to create issues and PRs.
See [DEVELOPMENT](DEVELOPMENT.md) for more details.  

## Benchmarks

The following command can be used to run the benchmarks.

*A local Redis instance is required.*

````
cargo bench --features benchmarks
````

## License
Licensed under either of

* Apache License, Version 2.0, (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)
at your option.

Each contributor agrees that his/her contribution covers both licenses.

## Credits

This crate is based on [redis-protocol](https://crates.io/crates/redis-protocol), developed by @aembke.
