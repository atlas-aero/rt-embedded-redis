//! This crate offers an asynchronous Redis client for `no_std` targets.
//! Both RESP2 and RESP3 protocol are supported.
//!
//! This crate consists of three parts:
//! * [network module](crate::network) for network details (connection handling, response management, etc.) + regular command client
//! * [commands module](crate::commands) for Redis command abstractions
//! * [subscription module][crate::subscription] for Redis subscription client
//!
//! ```no_run
//! # async fn example() {
//!# use core::str::FromStr;
//!# use core::net::SocketAddr;
//!# use std_embedded_nal_async::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//! let stack = Stack::default();
//! let clock = StandardClock::default();
//!
//! let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
//! let mut connection_handler = ConnectionHandler::resp2(server_address);
//! let client = connection_handler.connect(&stack, Some(&clock)).await.unwrap();
//!
//! let future = client.set("key", "value").await.unwrap();
//! let response = future.wait().await.unwrap();
//! # }
//! ```
#![cfg_attr(all(not(test), not(feature = "mock")), no_std)]
#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "benchmarks", feature(test))]
#[cfg(feature = "benchmarks")]
extern crate test;

extern crate alloc;
extern crate core;

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

/// # Connection and regular Client logic
///
/// ## Connection handling
///
/// Redis connection is managed by [ConnectionHandler](crate::network::ConnectionHandler).
/// Both [RESP2](https://redis.io/docs/reference/protocol-spec/) and [RESP3](https://github.com/antirez/RESP3/blob/master/spec.md) protocol
/// are supported.
///
/// Creating a new connection requires the following two things:
/// * A network stack implementing [embedded-nal-async](<https://docs.rs/embedded-nal-async/latest/embedded_nal_async/>)
/// * A clock implementing [embedded-time](<https://docs.rs/embedded-time/latest/embedded_time/>). Optional if no Timeout is configured.
/// ```no_run
/// # async fn example() {
///# use core::str::FromStr;
///# use core::net::SocketAddr;
///# use std_embedded_nal_async::Stack;
///# use std_embedded_time::StandardClock;
///# use embedded_redis::network::ConnectionHandler;
///#
/// let network_stack = Stack::default();
/// let clock = StandardClock::default();
///
/// // RESP2 protocol
/// let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
/// let _client = connection_handler.connect(&network_stack, Some(&clock)).await.unwrap();
///
/// // RESP3 protocol
/// let mut connection_handler = ConnectionHandler::resp3(SocketAddr::from_str("127.0.0.1:6379").unwrap());
/// let _client = connection_handler.connect(&network_stack, Some(&clock)).await.unwrap();
/// # }
/// ```
///
/// Each `connect()` call opens a new connection owned by the returned client.
///
/// ### Authentication
///
/// Authentication is done in the following way:
/// ```no_run
/// # async fn example() {
///# use core::str::FromStr;
///# use core::net::SocketAddr;
///# use std_embedded_nal_async::Stack;
///# use std_embedded_time::StandardClock;
///# use embedded_redis::network::{ConnectionHandler, Credentials};
///#
///# let network_stack = Stack::default();
///# let clock = StandardClock::default();
///#
///# let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
/// // Password only authentication
/// let mut connection_handler = ConnectionHandler::resp2(server_address);
/// connection_handler.auth(Credentials::password_only("secret123!"));
///
/// # let _client = connection_handler.connect(&network_stack, Some(&clock)).await;
///# let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
///
/// // ACL based authentication
/// let mut connection_handler = ConnectionHandler::resp2(server_address);
/// connection_handler.auth(Credentials::acl("user01", "secret123!"));
/// # let _client = connection_handler.connect(&network_stack, Some(&clock)).await;
/// # }
/// ```
/// ### Timeout
///
/// The client includes a timeout mechanism. This allows setting a time limit for responses from the Redis server:
///
/// ```no_run
/// # async fn example() {
///# use core::str::FromStr;
///# use core::net::SocketAddr;
///# use std_embedded_nal_async::Stack;
///# use std_embedded_time::StandardClock;
///# use embedded_redis::network::{ConnectionHandler, Credentials};
///# use embedded_time::duration::Extensions;
///#
///# let network_stack = Stack::default();
///# let clock = StandardClock::default();
///#
///# let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
/// let mut connection_handler = ConnectionHandler::resp2(server_address);
/// connection_handler.timeout(500_000.microseconds());
/// # let _client = connection_handler.connect(&network_stack, Some(&clock)).await.unwrap();
/// # }
/// ```
/// ### Ping
///
/// Optionally, the PING command can also be used to test the connection.
/// PING is then used to verify every newly opened connection.
///
/// It is recommended to use this option only if a Timeout is configured.
///
/// ```no_run
/// # async fn example() {
///# use core::str::FromStr;
///# use core::net::SocketAddr;
///# use std_embedded_nal_async::Stack;
///# use std_embedded_time::StandardClock;
///# use embedded_redis::network::{ConnectionHandler, Credentials};
///# use embedded_time::duration::Extensions;
///#
///# let network_stack = Stack::default();
///# let clock = StandardClock::default();
///#
///# let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
/// let mut connection_handler = ConnectionHandler::resp2(server_address);
/// connection_handler.timeout(500_000.microseconds());
/// connection_handler.use_ping();
/// # let _client = connection_handler.connect(&network_stack, Some(&clock)).await.unwrap();
/// # let _client = connection_handler.connect(&network_stack, Some(&clock)).await.unwrap();
/// # }
/// ```
///
/// ### Memory optimization
///
/// The following parameters can be used to optimize memory usage respectively to improve heap allocation.
/// The right parameters can also protect against DOS scenarios, when the received data can
/// potentially exceed the memory resources.
/// See [MemoryParameters](crate::network::MemoryParameters) for more details.
///
/// ````no_run
/// # async fn example() {
///# use core::str::FromStr;
///# use core::net::SocketAddr;
///# use std_embedded_nal_async::Stack;
///# use std_embedded_time::StandardClock;
///# use embedded_redis::commands::set::SetCommand;
///# use embedded_redis::network::{ConnectionHandler, MemoryParameters};
///#
///# let stack = Stack::default();
///# let clock = StandardClock::default();
///#
///# let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
/// let mut connection_handler = ConnectionHandler::resp3(server_address);
///
/// connection_handler.memory(MemoryParameters {
///     buffer_size: 512,
///     frame_capacity: 4,
///     memory_limit: Some(4096)
/// });
///
///# let client = connection_handler.connect(&stack, Some(&clock)).await.unwrap();
/// # }
/// ````
///
/// ### Concurrency
///
/// A client owns one connection. Access to that connection is serialized asynchronously, so
/// multiple response futures may be polled concurrently on the same executor. Whether a client
/// can be moved between threads depends on the concrete connection, clock, and protocol types.
///
/// ## Asynchronous response management
///
/// Redis server responses are managed as [Future](crate::network::Future). Sending several
/// commands before awaiting their responses enables pipelining, and responses can be consumed in
/// any order:
/// ```no_run
/// # async fn example() {
///# use core::str::FromStr;
///# use core::net::SocketAddr;
///# use std_embedded_nal_async::Stack;
///# use std_embedded_time::StandardClock;
///# use embedded_redis::commands::set::SetCommand;
///# use embedded_redis::network::ConnectionHandler;
///#
///# let stack = Stack::default();
///# let clock = StandardClock::default();
///#
///# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
///# let client = connection_handler.connect(&stack, Some(&clock)).await.unwrap();
///#
/// let future1 = client.set("key", "value").await.unwrap();
/// let future2 = client.set("other", "key").await.unwrap();
///
/// let _ = future2.wait().await;
/// let _ = future1.wait().await;
/// # }
/// ```
///
/// ### Ready
/// The asynchronous `ready()` method waits until the corresponding response arrives or an error
/// occurs. Errors are retained and returned by the subsequent `wait()` call.
/// ```no_run
/// # async fn example() {
///# use core::str::FromStr;
///# use core::net::SocketAddr;
///# use std_embedded_nal_async::Stack;
///# use std_embedded_time::StandardClock;
///# use embedded_redis::commands::set::SetCommand;
///# use embedded_redis::network::ConnectionHandler;
///#
///# let stack = Stack::default();
///# let clock = StandardClock::default();
///#
///# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
///# let client = connection_handler.connect(&stack, Some(&clock)).await.unwrap();
///#
/// let mut future = client.set("key", "value").await.unwrap();
///
/// if future.ready().await {
///    let _ = future.wait().await;
/// }
/// # }
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
///
/// ### Clean state
///
/// If futures are dropped without being awaited, `close().await` consumes their pending responses
/// before the client is dropped. Dropping the client itself closes its owned connection.
///
/// ````no_run
/// # async fn example() {
///# use core::str::FromStr;
///# use core::net::SocketAddr;
///# use std_embedded_nal_async::Stack;
///# use std_embedded_time::StandardClock;
///# use embedded_redis::commands::set::SetCommand;
///# use embedded_redis::network::ConnectionHandler;
///#
///# let stack = Stack::default();
///# let clock = StandardClock::default();
///#
///# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
///# let client = connection_handler.connect(&stack, Some(&clock)).await.unwrap();
///#
/// let _ = client.set("key", "value").await;
/// client.close().await;
/// # }
/// ````
pub mod network;
pub mod subscription;
