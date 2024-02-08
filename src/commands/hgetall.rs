//! Abstraction of HGETALL command.
//!
//! For general information about this command, see the [Redis documentation](<https://redis.io/commands/hgetall/>).
//!
//! # Using command object
//! ```
//!# use core::str::FromStr;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::builder::CommandBuilder;
//!# use embedded_redis::commands::hgetall::HashGetAllCommand;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//! let mut stack = Stack::default();
//! let clock = StandardClock::default();
//!
//! let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//! let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//! client.hset("test_all_hash", "color", "green").unwrap().wait().unwrap();
//! client.hset("test_all_hash", "material", "wood").unwrap().wait().unwrap();
//!
//! let command = HashGetAllCommand::new("test_all_hash");
//! let response = client.send(command).unwrap().wait().unwrap().unwrap();
//!
//! assert_eq!("green", response.get_str("color").unwrap());
//! assert_eq!("wood", response.get_str("material").unwrap());
//! ```
//!
//! # Missing key or field
//! In case key or field is missing. [None] is returned.
//! ```
//!# use core::str::FromStr;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::builder::CommandBuilder;
//!# use embedded_redis::commands::hgetall::HashGetAllCommand;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let mut stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//!# let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!#
//! let command = HashGetAllCommand::new("not_existing");
//! let response = client.send(command).unwrap().wait().unwrap();
//!
//! assert!(response.is_none())
//! ```
//!
//! # Shorthand
//! [Client](Client#method.hgetall) provides a shorthand method for this command.
//! ```
//!# use core::str::FromStr;
//!# use bytes::Bytes;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::hset::HashSetCommand;
//!# use embedded_redis::commands::set::SetCommand;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let mut stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//!# let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!#
//!# let _ = client.send(HashSetCommand::new("multi_hash_key", "first_field", "green")).unwrap().wait();
//!# let _ = client.send(HashSetCommand::new("multi_hash_key", "second_field", "wood")).unwrap().wait();
//!#
//! // Using &str arguments
//! let response = client.hgetall("multi_hash_key").unwrap().wait().unwrap().unwrap();
//! assert_eq!("green", response.get_str("first_field").unwrap());
//! assert_eq!("wood", response.get_str("second_field").unwrap());
//!
//! // Using String arguments
//! let _ = client.hgetall("multi_hash_key".to_string());
//!
//! // Using Bytes arguments
//! let _ = client.hgetall(Bytes::from_static(b"multi_hash_key"));
//! ```
use crate::commands::auth::AuthCommand;
use crate::commands::builder::{CommandBuilder, ToBytesMap};
use crate::commands::hello::HelloCommand;
use crate::commands::{Command, ResponseTypeError};
use crate::network::protocol::Protocol;
use crate::network::{Client, CommandErrors, Future};
use alloc::collections::BTreeMap;
use bytes::Bytes;
use embedded_nal::TcpClientStack;
use embedded_time::Clock;

/// Abstraction for HGETALL command
pub struct HashGetAllCommand {
    /// Hash key
    key: Bytes,
}

impl HashGetAllCommand {
    pub fn new<K>(key: K) -> Self
    where
        Bytes: From<K>,
    {
        Self { key: key.into() }
    }
}

pub struct HashResponse {
    /// Field/Value map
    inner: BTreeMap<Bytes, Bytes>,
}

impl HashResponse {
    /// Extracts inner map
    #[allow(clippy::wrong_self_convention)]
    pub fn to_map(self) -> BTreeMap<Bytes, Bytes> {
        self.inner
    }

    /// Returns the given field as &str. Returns None in case field is missing or value has invalid UTF8 encoding
    pub fn get_str<F>(&self, field: F) -> Option<&str>
    where
        Bytes: From<F>,
    {
        let field: Bytes = field.into();

        match self.inner.get(&field) {
            None => None,
            Some(value) => match core::str::from_utf8(value) {
                Ok(value) => Some(value),
                Err(_) => None,
            },
        }
    }
}

impl<F> Command<F> for HashGetAllCommand
where
    F: From<CommandBuilder> + ToBytesMap,
{
    type Response = Option<HashResponse>;

    fn encode(&self) -> F {
        CommandBuilder::new("HGETALL").arg(&self.key).into()
    }

    fn eval_response(&self, frame: F) -> Result<Self::Response, ResponseTypeError> {
        let map = frame.to_map();

        if map.is_none() {
            return Err(ResponseTypeError {});
        }

        if map.as_ref().unwrap().is_empty() {
            return Ok(None);
        }

        Ok(Some(HashResponse { inner: map.unwrap() }))
    }
}

impl<'a, N: TcpClientStack, C: Clock, P: Protocol> Client<'a, N, C, P>
where
    AuthCommand: Command<<P as Protocol>::FrameType>,
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    /// Shorthand for [HashGetAllCommand]
    pub fn hgetall<K>(&'a self, key: K) -> Result<Future<'a, N, C, P, HashGetAllCommand>, CommandErrors>
    where
        <P as Protocol>::FrameType: ToBytesMap,
        <P as Protocol>::FrameType: From<CommandBuilder>,
        Bytes: From<K>,
    {
        self.send(HashGetAllCommand::new(key))
    }
}
