//! Abstraction of GET command.
//!
//! For general information about this command, see the [Redis documentation](<https://redis.io/commands/get/>).
//!
//! # Basic usage
//! In case of existing key [`Some(GetResponse)`](GetResponse) is returned.
//! ```
//!# use core::str::FromStr;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::get::GetCommand;
//!# use embedded_redis::commands::set::SetCommand;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//! let mut stack = Stack::default();
//! let clock = StandardClock::default();
//!
//! let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//! let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!#
//!# let _ = client.send(SetCommand::new("test_key", "test_value")).unwrap().wait();
//!
//! let command = GetCommand::static_key("test_key");
//! let response = client.send(command).unwrap().wait().unwrap().unwrap();
//! assert_eq!("test_value", response.as_str().unwrap())
//! ```
//! # Missing key (NIL/NULL response)
//! In case of missing key `None` is returned
//! ```
//!# use core::str::FromStr;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::get::GetCommand;
//!# use embedded_redis::commands::set::SetCommand;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let mut stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//!# let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!#
//! let command = GetCommand::static_key("missing_key");
//! let response = client.send(command).unwrap().wait().unwrap();
//! assert!(response.is_none())
//! ```
//! # Using Bytes
//! For best performance (instead of &str or String cloning), especially with large amounts of data, it is recommended to use [Bytes](<https://docs.rs/bytes/latest/bytes/>).
//! ```
//!# use core::str::FromStr;
//!# use bytes::Bytes;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::get::GetCommand;
//!# use embedded_redis::commands::set::SetCommand;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let mut stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//!# let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!#
//!# let _ = client.send(SetCommand::new("test_key", "test_value")).unwrap().wait();
//!#
//! // Using Bytes object as key
//! let command = GetCommand::new(Bytes::from_static("large_key".as_bytes()));
//!
//! // Using response as Bytes
//! let command = GetCommand::new("test_key");
//! let response = client.send(command).unwrap().wait().unwrap().unwrap();
//! let _response_bytes = response.to_bytes();
//! ```
//! # Shorthand
//! [Client](Client#method.get) provides a shorthand method for this command.
//! ```
//!# use core::str::FromStr;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::set::SetCommand;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let mut stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//!# let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!#
//!# let _ = client.send(SetCommand::new("test_key", "test_value")).unwrap().wait();
//!#
//! let response = client.get("test_key").unwrap().wait().unwrap().unwrap();
//! assert_eq!("test_value", response.as_str().unwrap())
//! ```
use crate::commands::auth::AuthCommand;
use crate::commands::builder::{CommandBuilder, IsNullFrame, ToStringBytes};
use crate::commands::hello::HelloCommand;
use crate::commands::Command;
use crate::network::client::{Client, CommandErrors};
use crate::network::future::Future;
use crate::network::protocol::Protocol;
use alloc::string::String;
use bytes::Bytes;
use embedded_nal::TcpClientStack;
use embedded_time::Clock;

///Abstraction of GET command.
pub struct GetCommand {
    key: Bytes,
}

impl GetCommand {
    pub fn new<K>(key: K) -> Self
    where
        Bytes: From<K>,
    {
        GetCommand { key: key.into() }
    }

    /// Create from static key
    pub fn static_key(key: &'static str) -> Self {
        Self {
            key: Bytes::from_static(key.as_bytes()),
        }
    }
}

///Abstraction of GET response
pub struct GetResponse {
    inner: Bytes,
}

impl GetResponse {
    pub fn new(inner: Bytes) -> Self {
        GetResponse { inner }
    }

    /// Extracts inner value
    pub fn to_bytes(self) -> Bytes {
        self.inner
    }

    /// Tries converting to String by copy, returns None in case of error (wrong UTF8 encoding)
    pub fn as_string(&self) -> Option<String> {
        let result = String::from_utf8(self.inner.to_vec());
        if result.is_err() {
            return None;
        }

        Some(result.unwrap())
    }

    /// Returns a &str to inner data, returns None in case of invalid UTF8 encoding
    pub fn as_str(&self) -> Option<&str> {
        let result = core::str::from_utf8(self.inner.as_ref());
        if result.is_err() {
            return None;
        }

        Some(result.unwrap())
    }
}

impl<F> Command<F> for GetCommand
where
    F: From<CommandBuilder> + IsNullFrame + ToStringBytes,
{
    type Response = Option<GetResponse>;

    fn encode(&self) -> F {
        CommandBuilder::new("GET").arg(&self.key).into()
    }

    fn eval_response(&self, frame: F) -> Result<Self::Response, ()> {
        if frame.is_null_frame() {
            return Ok(None);
        }

        Ok(Some(GetResponse::new(frame.to_string_bytes().ok_or(())?)))
    }
}

impl<'a, N: TcpClientStack, C: Clock, P: Protocol> Client<'a, N, C, P>
where
    AuthCommand: Command<<P as Protocol>::FrameType>,
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    /// Shorthand for [GetCommand]
    pub fn get<K>(&'a self, key: K) -> Result<Future<'a, N, C, P, GetCommand>, CommandErrors>
    where
        <P as Protocol>::FrameType: ToStringBytes,
        <P as Protocol>::FrameType: IsNullFrame,
        <P as Protocol>::FrameType: From<CommandBuilder>,
        Bytes: From<K>,
    {
        self.send(GetCommand::new(key))
    }
}
