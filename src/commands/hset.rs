//! Abstraction of HSET command.
//!
//! For general information about this command, see the [Redis documentation](<https://redis.io/commands/hset/>).
//!
//! # Using command object
//! ```
//!# use core::str::FromStr;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::builder::CommandBuilder;
//! use embedded_redis::commands::hset::HashSetCommand;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//! let mut stack = Stack::default();
//! let clock = StandardClock::default();
//!
//! let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//! let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!# client.send(CommandBuilder::new("DEL").arg_static("my_hash").to_command()).unwrap().wait().unwrap();
//!
//! let command = HashSetCommand::new("my_hash", "color", "green");
//! let response = client.send(command).unwrap().wait().unwrap();
//!
//! // Returns the number of added fields
//! assert_eq!(1, response)
//! ```
//! # Setting multiple fields at once
//! ```
//!# use core::str::FromStr;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::builder::CommandBuilder;
//!# use embedded_redis::commands::hset::HashSetCommand;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let mut stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//!# let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!# client.send(CommandBuilder::new("DEL").arg_static("my_hash").to_command()).unwrap().wait().unwrap();
//!#
//! let command = HashSetCommand::multiple("my_hash".into(), [
//!     ("color".into(), "green".into()),
//!     ("material".into(), "stone".into())
//! ]);
//! let response = client.send(command).unwrap().wait().unwrap();
//!
//! // Returns the number of added fields
//! assert_eq!(2, response)
//! ```
//! # Shorthand
//! [Client](Client#method.hset) provides a shorthand method for this command.
//! ```
//!# use core::str::FromStr;
//!# use bytes::Bytes;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let mut stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//!# let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//! // Using &str arguments
//! let _ = client.hset("hash", "field", "value");
//!
//! // Using String arguments
//! let _ = client.hset("hash".to_string(), "field".to_string(), "value".to_string());
//!
//! // Using Bytes arguments
//! let _ = client.hset(Bytes::from_static(b"hash"), Bytes::from_static(b"field"), Bytes::from_static(b"value"));
//! ```
use crate::commands::auth::AuthCommand;
use crate::commands::builder::{CommandBuilder, ToInteger};
use crate::commands::hello::HelloCommand;
use crate::commands::{Command, ResponseTypeError};
use crate::network::protocol::Protocol;
use crate::network::{Client, CommandErrors, Future};
use bytes::Bytes;
use embedded_nal::TcpClientStack;
use embedded_time::Clock;

/// Abstraction of HSET command
pub struct HashSetCommand<const N: usize> {
    /// Hash key
    key: Bytes,

    /// Field/Value paris
    fields: [(Bytes, Bytes); N],
}

impl HashSetCommand<1> {
    pub fn new<K, F, V>(key: K, field: F, value: V) -> Self
    where
        Bytes: From<K>,
        Bytes: From<F>,
        Bytes: From<V>,
    {
        Self {
            key: key.into(),
            fields: [(field.into(), value.into())],
        }
    }
}

impl<const N: usize> HashSetCommand<N> {
    /// Constructs a new command with multiple field/value paris
    pub fn multiple(key: Bytes, fields: [(Bytes, Bytes); N]) -> Self {
        Self { key, fields }
    }
}

impl<F: From<CommandBuilder> + ToInteger, const N: usize> Command<F> for HashSetCommand<N> {
    type Response = i64;

    fn encode(&self) -> F {
        let mut builder = CommandBuilder::new("HSET").arg(&self.key);

        for (field, value) in &self.fields {
            builder = builder.arg(field).arg(value);
        }

        builder.into()
    }

    fn eval_response(&self, frame: F) -> Result<Self::Response, ResponseTypeError> {
        frame.to_integer().ok_or(ResponseTypeError {})
    }
}

impl<'a, N: TcpClientStack, C: Clock, P: Protocol> Client<'a, N, C, P>
where
    AuthCommand: Command<<P as Protocol>::FrameType>,
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    /// Shorthand for [HashSetCommand]
    /// For setting multiple fields, use [HashSetCommand] directly instead
    pub fn hset<K, F, V>(
        &'a self,
        key: K,
        field: F,
        value: V,
    ) -> Result<Future<'a, N, C, P, HashSetCommand<1>>, CommandErrors>
    where
        Bytes: From<K>,
        Bytes: From<F>,
        Bytes: From<V>,
        <P as Protocol>::FrameType: ToInteger,
        <P as Protocol>::FrameType: From<CommandBuilder>,
    {
        self.send(HashSetCommand::new(key, field, value))
    }
}
