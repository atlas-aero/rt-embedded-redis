//! This crate offers a non-blocking Redis Client for no_std targets.
//! Both RESP2 and RESP3 protocol are supported.
//!
//! This crate consists of two parts:
//! * [network module](crate::network) for network details (connection handling, response management, etc.)
//! * [commands module](crate::commands) for Redis command abstractions
//!
//! ```
//!# use core::str::FromStr;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//! let mut stack = Stack::default();
//! let clock = StandardClock::default();
//!
//! let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
//! let mut connection_handler = ConnectionHandler::resp2(server_address);
//! let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!
//! let future = client.set("key", "value").unwrap();
//! let response = future.wait().unwrap();
//! ```
#![cfg_attr(all(not(test), not(feature = "mock")), no_std)]
#![cfg_attr(feature = "strict", deny(warnings))]

extern crate alloc;

/// # Redis command abstractions
///
/// This crates includes abstractions for some Redis commands like
/// [AUTH](crate::commands::auth),
/// [HELLO](crate::commands::hello),
/// [GET](crate::commands::set),
/// [SET](crate::commands::set),
/// [PUBLISH](crate::commands::publish), ...
///
/// Each abstraction is implementing the [Command](crate::commands::Command) trait.
///
/// For executing arbitrary (not yet implemented) commands, [CustomCommand](crate::commands::custom)
/// may be used. As alternative you can create new commands by implementing the [Command](crate::commands::Command) trait.
///
/// *Please consider contributing new command abstractions*.
pub mod commands;

/// # Connection and Client logic
///
/// ## Connection handling
///
/// Redis connection is managed by [ConnectionHandler](crate::network::ConnectionHandler).
/// Both [RESP2](https://redis.io/docs/reference/protocol-spec/) and [RESP3](https://github.com/antirez/RESP3/blob/master/spec.md) protocol
/// are supported.
///
/// Creating a new connection requires the following two things:
/// * A network stack implementing [embedded-nal](<https://docs.rs/embedded-nal/latest/embedded_nal/>)
/// * A clock implementing [embedded-time](<https://docs.rs/embedded-time/latest/embedded_time/>). Optional if no Timeout is configured.
/// ```
///# use core::str::FromStr;
///# use embedded_nal::SocketAddr;
///# use std_embedded_nal::Stack;
///# use std_embedded_time::StandardClock;
///# use embedded_redis::network::ConnectionHandler;
///#
/// let mut network_stack = Stack::default();
/// let clock = StandardClock::default();
///
/// // RESP2 protocol
/// let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
/// let _client = connection_handler.connect(&mut network_stack, Some(&clock)).unwrap();
///
/// // RESP3 protocol
/// let mut connection_handler = ConnectionHandler::resp3(SocketAddr::from_str("127.0.0.1:6379").unwrap());
/// let _client = connection_handler.connect(&mut network_stack, Some(&clock)).unwrap();
/// ```
///
/// ConnectionHandler is caching the connection, so later recreation of new Clients is cheap.
///
/// ### Authentication
///
/// Authentication is done in the following way:
/// ```
///# use core::str::FromStr;
///# use embedded_nal::SocketAddr;
///# use std_embedded_nal::Stack;
///# use std_embedded_time::StandardClock;
///# use embedded_redis::network::{ConnectionHandler, Credentials};
///#
///# let mut network_stack = Stack::default();
///# let clock = StandardClock::default();
///#
///# let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
/// // Password only authentication
/// let mut connection_handler = ConnectionHandler::resp2(server_address);
/// connection_handler.auth(Credentials::password_only("secret123!"));
///
/// # let _client = connection_handler.connect(&mut network_stack, Some(&clock));
///# let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
///
/// // ACL based authentication
/// let mut connection_handler = ConnectionHandler::resp2(server_address);
/// connection_handler.auth(Credentials::acl("user01", "secret123!"));
/// # let _client = connection_handler.connect(&mut network_stack, Some(&clock));
/// ```
/// ### Timeout
///
/// The client includes a timeout mechanism. This allows setting a time limit for responses from the Redis server:
///
/// ```
///# use core::str::FromStr;
///# use embedded_nal::SocketAddr;
///# use std_embedded_nal::Stack;
///# use std_embedded_time::StandardClock;
///# use embedded_redis::network::{ConnectionHandler, Credentials};
///# use embedded_time::duration::Extensions;
///#
///# let mut network_stack = Stack::default();
///# let clock = StandardClock::default();
///#
///# let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
/// let mut connection_handler = ConnectionHandler::resp2(server_address);
/// connection_handler.timeout(500_000.microseconds());
/// # let _client = connection_handler.connect(&mut network_stack, Some(&clock)).unwrap();
/// ```
/// ### Ping
///
/// Optionally, the PING command can also be used to test the connection.
/// PING is then used every time `connect()` is called after the socket has been cached.
///
/// It is recommended to use this option only if a Timeout is configured.
///
/// ```
///# use core::str::FromStr;
///# use embedded_nal::SocketAddr;
///# use std_embedded_nal::Stack;
///# use std_embedded_time::StandardClock;
///# use embedded_redis::network::{ConnectionHandler, Credentials};
///# use embedded_time::duration::Extensions;
///#
///# let mut network_stack = Stack::default();
///# let clock = StandardClock::default();
///#
///# let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
/// let mut connection_handler = ConnectionHandler::resp2(server_address);
/// connection_handler.timeout(500_000.microseconds());
/// connection_handler.use_ping();
/// # let _client = connection_handler.connect(&mut network_stack, Some(&clock)).unwrap();
/// # let _client = connection_handler.connect(&mut network_stack, Some(&clock)).unwrap();
/// ```
///
/// ### Concurrency
///
/// While the Client is not Send, the connection handler is.
/// The handler is designed with the approach that the creation of new clients is cheap.
/// Thus, the use of short-lived clients in concurrent applications is not a problem.
///
/// ## Non-blocking response management
///
/// Redis server responses are managed as [Future](crate::network::Future). This allows executing multiple commands non-blocking
/// simultaneously* and handle responses in any order at any point in time:
/// ```
///# use core::str::FromStr;
///# use embedded_nal::SocketAddr;
///# use std_embedded_nal::Stack;
///# use std_embedded_time::StandardClock;
///# use embedded_redis::commands::set::SetCommand;
///# use embedded_redis::network::ConnectionHandler;
///#
///# let mut stack = Stack::default();
///# let clock = StandardClock::default();
///#
///# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
///# let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
///#
/// let future1 = client.set("key", "value").unwrap();
/// let future2 = client.set("other", "key").unwrap();
///
/// let _ = future2.wait();
/// let _ = future1.wait();
/// ```
///
/// ### Ready
/// In order to check whether a future is ready, (the corresponding response has arrived),
/// the method `ready()` can be used.
/// If `ready()` returns true, then next call to `wait()` is not expected to block.
/// ```
///# use core::str::FromStr;
///# use embedded_nal::SocketAddr;
///# use std_embedded_nal::Stack;
///# use std_embedded_time::StandardClock;
///# use embedded_redis::commands::set::SetCommand;
///# use embedded_redis::network::ConnectionHandler;
///#
///# let mut stack = Stack::default();
///# let clock = StandardClock::default();
///#
///# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
///# let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
///#
/// let mut future = client.set("key", "value").unwrap();
///
/// if future.ready() {
///    let _ = future.wait();
/// }
/// ```
///
/// ### Response type
///
/// Response type dependents on executed command abstractions, e.g. [GetResponse](crate::commands::get::GetResponse)
/// in case of [GET command](crate::commands::get).
///
/// ### Timeout error
///
/// In the event of a timeout error, all remaining futures will be invalidated, as the assignment of
/// responses can no longer be guaranteed. In case of a invalidated future [InvalidFuture](crate::network::CommandErrors::InvalidFuture)
/// error is returned when calling `wait()`.
pub mod network;
