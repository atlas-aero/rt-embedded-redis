//! # Subscription client
//!
//! This crates supports subscribing to one or multiple channels. (s. [Redis Pub/Sub](https://redis.io/docs/manual/pubsub/)).
//!
//! A regular client can be turned to a [Subscription] in the following way.
//!
//! ```
//!# use core::str::FromStr;
//!# use core::net::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let mut stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
//!# let mut connection_handler = ConnectionHandler::resp3(server_address);
//! let client = connection_handler
//!                 .connect(&mut stack, Some(&clock)).unwrap()
//!                 .subscribe(["first_channel".into(), "second_channel".into()])
//!                 .unwrap();
//! ```
//!
//! If the subscriptions fails, it's recommended to close the connection, as a the
//! state is undefined. A further reuse of the connection could cause subsequent errors.
//!
//! ## Receiving messages
//!
//! Messages can be received using the `receive()` method. Which returns [Some(Message)](Message) in case a message is pending.
//!
//! ```
//!# use core::str::FromStr;
//!# use std::{thread, time};
//!# use core::net::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# thread::spawn(|| {
//!#     let mut stack = Stack::default();
//!#     let clock = StandardClock::default();
//!#
//!#     let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
//!#     let mut connection_handler = ConnectionHandler::resp3(server_address);
//!#     let mut  client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!#
//!#     loop {
//!#         client.publish("first_channel", "example payload").unwrap().wait().unwrap();
//!#         thread::sleep(time::Duration::from_millis(10));
//!#     }
//!# });
//!#
//!# let mut stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
//!# let mut connection_handler = ConnectionHandler::resp3(server_address);
//!# let mut  client = connection_handler
//!#                 .connect(&mut stack, Some(&clock)).unwrap()
//!#                 .subscribe(["first_channel".into(), "second_channel".into()])
//!#                 .unwrap();
//!#
//! loop {
//!     let message = client.receive().unwrap();
//!
//!     if let Some(message) = message {
//!         assert_eq!("first_channel", core::str::from_utf8(&message.channel[..]).unwrap());
//!         assert_eq!("example payload", core::str::from_utf8(&message.payload[..]).unwrap());
//!         break;
//!     }
//! }
//! ```
//!
//! ## Unsubscribing
//!
//! To leave a clean connection state, unsubscribe from all channels at the end.
//!
//! ```
//!# use core::str::FromStr;
//!# use core::net::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let mut stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
//!# let mut connection_handler = ConnectionHandler::resp3(server_address);
//!# let client = connection_handler
//!#                 .connect(&mut stack, Some(&clock)).unwrap()
//!#                 .subscribe(["first_channel".into(), "second_channel".into()])
//!#                 .unwrap();
//!#
//! client.unsubscribe().unwrap();
//! ```
//!
//! *Note: `unsubscribe()` is called automatically when the client is dropped*
pub use client::{Error, Message, Subscription};

pub(crate) mod client;
pub(crate) mod messages;

#[cfg(test)]
mod tests;
