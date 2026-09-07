//! Abstraction of KEYS command.
//!
//! Returns the keys matching a Redis glob-style pattern, such as `user:*` or `user:??`.
//! The pattern is passed unchanged to Redis. Keys are returned as binary-safe [Bytes] values;
//! no matches produce an empty vector. The order of the keys is unspecified.
//!
//! For general information about this command, see the [Redis documentation](https://redis.io/commands/keys/).
//! KEYS scans the entire database in O(N) time and can block Redis on large databases.
//!
//! # Using command object
//! ```
//! # async_std::task::block_on(async {
//! # use core::net::SocketAddr;
//! # use core::str::FromStr;
//! # use bytes::Bytes;
//! # use std_embedded_nal_async::Stack;
//! # use std_embedded_time::StandardClock;
//! use embedded_redis::commands::keys::KeysCommand;
//! use embedded_redis::network::ConnectionHandler;
//!
//! let stack = Stack::default();
//! let clock = StandardClock::default();
//! let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//! let client = connection_handler.connect(&stack, Some(&clock)).await.unwrap();
//! client.set("keys_example:one", "value").await.unwrap();
//!
//! let command = KeysCommand::new("keys_example:*");
//! let keys = client.send(command).await.unwrap();
//! assert!(keys.contains(&Bytes::from_static(b"keys_example:one")));
//! # });
//! ```
//!
//! # Shorthand
//! [Client::keys] accepts `&'static str`, `String`, or [Bytes] patterns and supports both RESP2 and RESP3.
//! ```
//! # async_std::task::block_on(async {
//! # use core::net::SocketAddr;
//! # use core::str::FromStr;
//! # use bytes::Bytes;
//! # use std_embedded_nal_async::Stack;
//! # use std_embedded_time::StandardClock;
//! # use embedded_redis::network::ConnectionHandler;
//! # let stack = Stack::default();
//! # let clock = StandardClock::default();
//! # let mut connection_handler = ConnectionHandler::resp3(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//! # let client = connection_handler.connect(&stack, Some(&clock)).await.unwrap();
//! let keys = client.keys("keys_example:*").await.unwrap();
//! let keys = client.keys("keys_example:*".to_string()).await.unwrap();
//! let keys = client.keys(Bytes::from_static(b"keys_example:*")).await.unwrap();
//! # });
//! ```
use crate::commands::auth::AuthCommand;
use crate::commands::builder::{CommandBuilder, ToBytesVec};
use crate::commands::hello::HelloCommand;
use crate::commands::{Command, ResponseTypeError};
use crate::network::protocol::Protocol;
use crate::network::{Client, CommandErrors};
use alloc::vec::Vec;
use bytes::Bytes;
use embedded_io_async::{Read, Write};
use embedded_time::Clock;

/// Abstraction of KEYS command.
pub struct KeysCommand {
    pattern: Bytes,
}

impl KeysCommand {
    /// Creates a command with a Redis glob-style pattern. Use `*` to match all keys.
    ///
    /// Accepts `&'static str`, `String`, or [Bytes], including binary patterns.
    pub fn new<K>(pattern: K) -> Self
    where
        Bytes: From<K>,
    {
        Self {
            pattern: pattern.into(),
        }
    }
}

impl<F> Command<F> for KeysCommand
where
    F: From<CommandBuilder> + ToBytesVec,
{
    /// Matching keys, or an empty vector when no keys match.
    type Response = Vec<Bytes>;

    fn encode(&self) -> F {
        CommandBuilder::new("KEYS").arg(&self.pattern).into()
    }

    fn eval_response(&self, frame: F) -> Result<Self::Response, ResponseTypeError> {
        frame.to_vec().ok_or(ResponseTypeError {})
    }
}

impl<'a, T: Read + Write, C: Clock, P: Protocol> Client<'a, T, C, P>
where
    AuthCommand: Command<<P as Protocol>::FrameType>,
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    /// Shorthand for [KeysCommand]. Returns matching keys, or an empty vector if none match.
    pub async fn keys<K>(&self, pattern: K) -> Result<Vec<Bytes>, CommandErrors>
    where
        <P as Protocol>::FrameType: ToBytesVec,
        <P as Protocol>::FrameType: From<CommandBuilder>,
        Bytes: From<K>,
    {
        self.send(KeysCommand::new(pattern)).await
    }
}
