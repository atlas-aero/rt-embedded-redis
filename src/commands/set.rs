//! Abstraction of SET command.
//!
//! For general information about this command, see the [Redis documentation](<https://redis.io/commands/set/>).
//!
//! # Basic usage
//! ```
//!# use core::str::FromStr;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::set::SetCommand;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//! let mut stack = Stack::default();
//! let clock = StandardClock::default();
//!
//! let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//! let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!
//! let command = SetCommand::new("key", "value");
//! let _ = client.send(command);
//! ```
//!
//! # Expiration (EX, PX, EXAT, PXAT)
//! Setting TTL can be achieved in the following way. Fore more details s. [ExpirationPolicy] enum.
//! ```
//!# use core::str::FromStr;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::set::{SetCommand, ExpirationPolicy};
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let mut stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//!# let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!#
//!  // Expires in 120 seconds
//!  let command = SetCommand::new("key", "value")
//!      .expires(ExpirationPolicy::Seconds(120));
//!# let _ = client.send(command);
//! ```
//! # Exclusive condition (NX/XX)
//! Manage set condition. Fore more details s. [Exclusivity] enum.
//!
//! Using this options affects the return type. s. [ExclusiveSetResponse]
//! ```
//!# use core::str::FromStr;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::set::{SetCommand, Exclusivity};
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let mut stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//!# let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!#
//!  // Just set the key if its not existing yet
//!  let command = SetCommand::new("key", "value")
//!      .set_exclusive(Exclusivity::SetIfMissing);
//!# let _ = client.send(command);
//! ```
//! # Return previous value (!GET)
//! Returns the previous value stored at the given key.
//!
//! Using this options affects the return type. s. [ReturnPreviousResponse]
//! ```
//!# use core::str::FromStr;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::set::{SetCommand};
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let mut stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//!# let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!#
//!  // Just set the key if its not existing yet
//!  let command = SetCommand::new("key", "value")
//!      .return_previous();
//!# let _ = client.send(command);
//! ```
//! # Shorthand
//! [Client](Client#method.set) provides a shorthand method for this command.
//! ```
//!# use core::str::FromStr;
//!# use bytes::Bytes;
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
//! // Using &str arguments
//! let _ = client.set("key", "value");
//!
//! // Using String arguments
//! let _ = client.set("key".to_string(), "value".to_string());
//!
//! // Using Bytes arguments
//! let _ = client.set(Bytes::from_static(b"key"), Bytes::from_static(b"value"));
//! ```

use crate::commands::auth::AuthCommand;
use crate::commands::builder::{CommandBuilder, IsNullFrame, ToStringBytes, ToStringOption};
use crate::commands::hello::HelloCommand;
use crate::commands::Command;
use crate::network::client::{Client, CommandErrors};
use crate::network::future::Future;
use crate::network::protocol::Protocol;
use alloc::string::ToString;
use bytes::Bytes;
use core::marker::PhantomData;
use embedded_nal::TcpClientStack;
use embedded_time::Clock;

pub enum ExpirationPolicy {
    /// Does not set and expiration option
    Never,
    /// EX option
    Seconds(usize),
    /// PX option
    Milliseconds(usize),
    /// EXAT option
    TimestampSeconds(usize),
    /// PXAT option
    TimestampMilliseconds(usize),
    /// KEEPTTL option
    Keep,
}

pub enum Exclusivity {
    None,
    /// NX option
    SetIfExists,
    /// XX option
    SetIfMissing,
}

pub struct SetCommand<R> {
    key: Bytes,
    value: Bytes,
    expiration: ExpirationPolicy,
    exclusivity: Exclusivity,

    /// GET option
    return_old_value: bool,

    response_type: PhantomData<R>,
}

impl SetCommand<ConfirmationResponse> {
    pub fn new<K, V>(key: K, value: V) -> Self
    where
        Bytes: From<K>,
        Bytes: From<V>,
    {
        SetCommand {
            key: key.into(),
            value: value.into(),
            expiration: ExpirationPolicy::Never,
            exclusivity: Exclusivity::None,
            return_old_value: false,
            response_type: PhantomData,
        }
    }

    /// Set expiration (TTL)
    pub fn expires(mut self, policy: ExpirationPolicy) -> SetCommand<ConfirmationResponse> {
        self.expiration = policy;
        self
    }

    /// Only set key if Exclusivity condition is met
    pub fn set_exclusive(self, option: Exclusivity) -> SetCommand<ExclusiveSetResponse> {
        SetCommand {
            key: self.key,
            value: self.value,
            expiration: self.expiration,
            exclusivity: option,
            return_old_value: self.return_old_value,
            response_type: PhantomData,
        }
    }
}

impl<R> SetCommand<R> {
    /// Returns the previous key by setting the GET option
    pub fn return_previous(self) -> SetCommand<ReturnPreviousResponse> {
        SetCommand {
            key: self.key,
            value: self.value,
            expiration: self.expiration,
            exclusivity: self.exclusivity,
            return_old_value: true,
            response_type: PhantomData,
        }
    }
}

/// Regular response if neither !GET or NX/XX option is set.
/// Indicates that SET operation was successful
pub type ConfirmationResponse = ();

/// Response if NX/XX option was set.
///
/// Some => SET was executed successfully.
/// None => Operation was not performed, as NX/XX condition was not met.
pub type ExclusiveSetResponse = Option<()>;

/// Response if !GET option is used.
///
/// Some => The old string value stored at key.
/// None => The key did not exist.
pub type ReturnPreviousResponse = Option<Bytes>;

impl<F> Command<F> for SetCommand<ConfirmationResponse>
where
    F: From<CommandBuilder> + ToStringOption,
{
    type Response = ConfirmationResponse;

    fn encode(&self) -> F {
        self.get_builder().into()
    }

    fn eval_response(&self, frame: F) -> Result<Self::Response, ()> {
        if frame.to_string_option().ok_or(())? != "OK" {
            return Err(());
        }

        Ok(())
    }
}

impl<F> Command<F> for SetCommand<ExclusiveSetResponse>
where
    F: From<CommandBuilder> + ToStringOption + IsNullFrame,
{
    type Response = ExclusiveSetResponse;

    fn encode(&self) -> F {
        self.get_builder().into()
    }

    fn eval_response(&self, frame: F) -> Result<Self::Response, ()> {
        if frame.is_null_frame() {
            return Ok(None);
        }

        if frame.to_string_option().ok_or(())? == "OK" {
            return Ok(Some(()));
        }

        Err(())
    }
}

impl<F> Command<F> for SetCommand<ReturnPreviousResponse>
where
    F: From<CommandBuilder> + IsNullFrame + ToStringBytes,
{
    type Response = ReturnPreviousResponse;

    fn encode(&self) -> F {
        self.get_builder().into()
    }

    fn eval_response(&self, frame: F) -> Result<Self::Response, ()> {
        if frame.is_null_frame() {
            return Ok(None);
        }

        Ok(Some(frame.to_string_bytes().ok_or(())?))
    }
}

impl<R> SetCommand<R> {
    /// General logic for building the command
    fn get_builder(&self) -> CommandBuilder {
        CommandBuilder::new("SET")
            .arg(&self.key)
            .arg(&self.value)
            .arg_static_option(self.expiration_unit())
            .arg_option(self.expiration_time().as_ref())
            .arg_static_option(self.exclusive_option())
            .arg_static_option(self.get_option())
    }

    /// Returns the expiration time unit argument
    fn expiration_unit(&self) -> Option<&'static str> {
        match self.expiration {
            ExpirationPolicy::Never => None,
            ExpirationPolicy::Seconds(_) => Some("EX"),
            ExpirationPolicy::Milliseconds(_) => Some("PX"),
            ExpirationPolicy::TimestampSeconds(_) => Some("EXAT"),
            ExpirationPolicy::TimestampMilliseconds(_) => Some("PXAT"),
            ExpirationPolicy::Keep => Some("KEEPTTL"),
        }
    }

    /// Returns the expiration time
    fn expiration_time(&self) -> Option<Bytes> {
        match self.expiration {
            ExpirationPolicy::Never => None,
            ExpirationPolicy::Seconds(seconds)
            | ExpirationPolicy::Milliseconds(seconds)
            | ExpirationPolicy::TimestampSeconds(seconds)
            | ExpirationPolicy::TimestampMilliseconds(seconds) => Some(seconds.to_string().into()),
            ExpirationPolicy::Keep => None,
        }
    }

    /// Returns the exclusivity argument
    fn exclusive_option(&self) -> Option<&'static str> {
        match self.exclusivity {
            Exclusivity::None => None,
            Exclusivity::SetIfExists => Some("XX"),
            Exclusivity::SetIfMissing => Some("NX"),
        }
    }

    fn get_option(&self) -> Option<&'static str> {
        if self.return_old_value {
            return Some("GET");
        }

        None
    }
}

impl<'a, N: TcpClientStack, C: Clock, P: Protocol> Client<'a, N, C, P>
where
    AuthCommand: Command<<P as Protocol>::FrameType>,
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    /// Shorthand for [SetCommand]
    /// For using options of SET command, use [SetCommand] directly instead
    pub fn set<K, V>(
        &'a self,
        key: K,
        value: V,
    ) -> Result<Future<'a, N, C, P, SetCommand<ConfirmationResponse>>, CommandErrors>
    where
        <P as Protocol>::FrameType: ToStringBytes,
        <P as Protocol>::FrameType: ToStringOption,
        <P as Protocol>::FrameType: IsNullFrame,
        <P as Protocol>::FrameType: From<CommandBuilder>,
        Bytes: From<K>,
        Bytes: From<V>,
    {
        self.send(SetCommand::new(key, value))
    }
}
