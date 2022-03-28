# Redis Client for no_std

This crate offers a non-blocking Redis Client for no_std targets.
Both RESP2 and RESP3 protocol are supported.

This crate consists of two parts:
* [network module](crate::network) for network details (connection handling, response management, etc.)
* [commands module](crate::commands) for Redis command abstractions



```rust
use core::str::FromStr;
use embedded_nal::SocketAddr;
use std_embedded_nal::Stack;
use std_embedded_time::StandardClock;
use embedded_redis::network::ConnectionHandler;

let mut stack = Stack::default();
let clock = StandardClock::default();

let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
let mut connection_handler = ConnectionHandler::resp2(server_address);
let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();

let future = client.set("key", "value").unwrap();
let response = future.wait().unwrap();
```

## Development

Any form of support is greatly appreciated. Feel free to create issues and PRs.
See [DEVELOPMENT](DEVELOPMENT.md) for more details.  

## License
Licensed under either of

* Apache License, Version 2.0, (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)
at your option.

Each contributor agrees that his/her contribution covers both licenses.