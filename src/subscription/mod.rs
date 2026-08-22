//! # Subscription client
//!
//! This crates supports subscribing to one or multiple channels. (s. [Redis Pub/Sub](https://redis.io/docs/manual/pubsub/)).
//!
//! A regular client can be turned to a [Subscription] in the following way.
//!
//! ```
//! # async_std::task::block_on(async {
//!# use core::str::FromStr;
//!# use core::net::SocketAddr;
//!# use std_embedded_nal_async::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
//!# let mut connection_handler = ConnectionHandler::resp3(server_address);
//! let client = connection_handler
//!                 .connect(&stack, Some(&clock)).await.unwrap()
//!                 .subscribe(["first_channel".into(), "second_channel".into()]).await
//!                 .unwrap();
//! # });
//! ```
//!
//! If subscribing fails, drop the client so its owned connection is closed; the server-side state
//! may be undefined.
//!
//! ## Receiving messages
//!
//! Messages can be received asynchronously using the `receive()` method. It waits until a
//! publish message arrives and then returns [Some(Message)](Message).
//!
//! ```
//! # async_std::task::block_on(async {
//!# use core::str::FromStr;
//!# use core::net::SocketAddr;
//!# use std_embedded_nal_async::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
//!# let connection_handler = ConnectionHandler::resp3(server_address);
//!# let mut client = connection_handler
//!#                 .connect(&stack, Some(&clock)).await.unwrap()
//!#                 .subscribe(["embedded_redis_doctest_channel".into()]).await
//!#                 .unwrap();
//!#
//!# let publisher_handler = ConnectionHandler::resp3(server_address);
//!# let publisher = publisher_handler.connect(&stack, Some(&clock)).await.unwrap();
//!# publisher
//!#     .publish("embedded_redis_doctest_channel", "example payload")
//!#     .await.unwrap().wait().await.unwrap();
//!#
//! let message = client.receive().await.unwrap().unwrap();
//! assert_eq!("embedded_redis_doctest_channel", core::str::from_utf8(&message.channel[..]).unwrap());
//! assert_eq!("example payload", core::str::from_utf8(&message.payload[..]).unwrap());
//!# client.unsubscribe().await.unwrap();
//! # });
//! ```
//!
//! ## Unsubscribing
//!
//! To leave a clean connection state, unsubscribe from all channels at the end.
//!
//! ```
//! # async_std::task::block_on(async {
//!# use core::str::FromStr;
//!# use core::net::SocketAddr;
//!# use std_embedded_nal_async::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
//!# let mut connection_handler = ConnectionHandler::resp3(server_address);
//!# let client = connection_handler
//!#                 .connect(&stack, Some(&clock)).await.unwrap()
//!#                 .subscribe(["first_channel".into(), "second_channel".into()]).await
//!#                 .unwrap();
//!#
//! client.unsubscribe().await.unwrap();
//! # });
//! ```
//!
//! Dropping a subscription closes its owned connection. Call `unsubscribe()` when the server's
//! confirmation is required before closing it.
pub use client::{Error, Message, Subscription};

pub(crate) mod client;
pub(crate) mod messages;

#[cfg(test)]
mod tests;
