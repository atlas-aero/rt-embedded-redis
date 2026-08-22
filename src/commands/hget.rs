//! Abstraction of HGET command.
//!
//! For general information about this command, see the [Redis documentation](<https://redis.io/commands/hget/>).
//!
//! # Using command object
//! ```
//! # async_std::task::block_on(async {
//!# use core::str::FromStr;
//!# use core::net::SocketAddr;
//!# use std_embedded_nal_async::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::builder::CommandBuilder;
//!# use embedded_redis::commands::hget::HashGetCommand;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//! let stack = Stack::default();
//! let clock = StandardClock::default();
//!
//! let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//! let client = connection_handler.connect(&stack, Some(&clock)).await.unwrap();
//! client.hset("test_hash", "color", "green").await.unwrap();
//!
//! let command = HashGetCommand::new("test_hash", "color");
//! let response = client.send(command).await.unwrap().unwrap();
//!
//! assert_eq!("green", response.as_str().unwrap())
//! # });
//! ```
//!
//! # Missing key or field
//! In case key or field is missing. [None] is returned.
//! ```
//! # async_std::task::block_on(async {
//!# use core::str::FromStr;
//!# use core::net::SocketAddr;
//!# use std_embedded_nal_async::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::builder::CommandBuilder;
//!# use embedded_redis::commands::hget::HashGetCommand;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//!# let client = connection_handler.connect(&stack, Some(&clock)).await.unwrap();
//!#
//! let command = HashGetCommand::new("not_existing", "field");
//! let response = client.send(command).await.unwrap();
//!
//! assert!(response.is_none())
//! # });
//! ```
//!
//! # Shorthand
//! [Client](Client#method.hget) provides a shorthand method for this command.
//! ```
//! # async_std::task::block_on(async {
//!# use core::str::FromStr;
//!# use bytes::Bytes;
//!# use core::net::SocketAddr;
//!# use std_embedded_nal_async::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::hset::HashSetCommand;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//!# let client = connection_handler.connect(&stack, Some(&clock)).await.unwrap();
//!#
//!# let _ = client.send(HashSetCommand::new("hash_key", "hash_field", "example")).await;
//!#
//! // Using &str arguments
//! let response = client.hget("hash_key", "hash_field").await.unwrap().unwrap();
//! assert_eq!("example", response.as_str().unwrap());
//!
//! // Using String arguments
//! let _ = client.hget("hash_key".to_string(), "hash_field".to_string()).await;
//!
//! // Using Bytes arguments
//! let _ = client.hget(Bytes::from_static(b"hash_key"), Bytes::from_static(b"hash_field")).await;
//! # });
//! ```
use crate::commands::auth::AuthCommand;
use crate::commands::builder::{CommandBuilder, IsNullFrame, ToStringBytes};
use crate::commands::get::GetResponse;
use crate::commands::hello::HelloCommand;
use crate::commands::{Command, ResponseTypeError};
use crate::network::protocol::Protocol;
use crate::network::{Client, CommandErrors};
use bytes::Bytes;
use embedded_io_async::{Read, Write};
use embedded_time::Clock;

/// Abstraction for HGET command
pub struct HashGetCommand {
    /// Hash key
    key: Bytes,

    /// Hash field to receive
    field: Bytes,
}

impl HashGetCommand {
    pub fn new<K, F>(key: K, field: F) -> Self
    where
        Bytes: From<K>,
        Bytes: From<F>,
    {
        Self {
            key: key.into(),
            field: field.into(),
        }
    }
}

impl<F> Command<F> for HashGetCommand
where
    F: From<CommandBuilder> + IsNullFrame + ToStringBytes,
{
    type Response = Option<GetResponse>;

    fn encode(&self) -> F {
        CommandBuilder::new("HGET").arg(&self.key).arg(&self.field).into()
    }

    fn eval_response(&self, frame: F) -> Result<Self::Response, ResponseTypeError> {
        GetResponse::from_frame(frame)
    }
}

impl<'a, T: Read + Write, C: Clock, P: Protocol> Client<'a, T, C, P>
where
    AuthCommand: Command<<P as Protocol>::FrameType>,
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    /// Shorthand for [HashGetCommand]
    pub async fn hget<K, F>(&self, key: K, field: F) -> Result<Option<GetResponse>, CommandErrors>
    where
        <P as Protocol>::FrameType: ToStringBytes,
        <P as Protocol>::FrameType: IsNullFrame,
        <P as Protocol>::FrameType: From<CommandBuilder>,
        Bytes: From<K>,
        Bytes: From<F>,
    {
        self.send(HashGetCommand::new(key, field)).await
    }
}
