//! Abstraction of PUBLISH command.
//!
//! For general information about this command, see the [Redis documentation](<https://redis.io/commands/publish/>).
//!
//! # Using command object
//! ```
//!# use core::str::FromStr;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::publish::PublishCommand;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//! let mut stack = Stack::default();
//! let clock = StandardClock::default();
//!
//! let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//! let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!
//! let command = PublishCommand::new("channel", "message");
//! let response = client.send(command).unwrap().wait().unwrap();
//!
//! // Returns the number of clients that received the message
//! assert_eq!(0, response)
//! ```
//! # Shorthand
//! [Client](Client#method.publish) provides a shorthand method.
//! ```
//!# use core::str::FromStr;
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
//!#
//! let _ = client.publish("channel", "message");
//! ```
use crate::commands::auth::AuthCommand;
use crate::commands::builder::{CommandBuilder, ToInteger};
use crate::commands::hello::HelloCommand;
use crate::commands::{Command, ResponseTypeError};
use crate::network::client::{Client, CommandErrors};
use crate::network::future::Future;
use crate::network::protocol::Protocol;
use bytes::Bytes;
use embedded_nal::TcpClientStack;
use embedded_time::Clock;

/// Abstraction for PUBLISH command
pub struct PublishCommand {
    channel: Bytes,
    message: Bytes,
}

impl PublishCommand {
    pub fn new<C, M>(channel: C, message: M) -> Self
    where
        Bytes: From<C>,
        Bytes: From<M>,
    {
        PublishCommand {
            channel: channel.into(),
            message: message.into(),
        }
    }
}

impl<F> Command<F> for PublishCommand
where
    F: From<CommandBuilder> + ToInteger,
{
    /// the number of clients that received the message
    type Response = i64;

    fn encode(&self) -> F {
        CommandBuilder::new("PUBLISH").arg(&self.channel).arg(&self.message).into()
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
    /// Shorthand for [PublishCommand]
    pub fn publish<K, V>(
        &'a self,
        channel: K,
        message: V,
    ) -> Result<Future<'a, N, C, P, PublishCommand>, CommandErrors>
    where
        <P as Protocol>::FrameType: ToInteger,
        <P as Protocol>::FrameType: From<CommandBuilder>,
        Bytes: From<K>,
        Bytes: From<V>,
    {
        self.send(PublishCommand::new(channel, message))
    }
}
