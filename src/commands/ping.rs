//! Abstraction of PING command.
//!
//! For general information about this command, see the [Redis documentation](<https://redis.io/commands/ping/>).
//!
//! # Basic usage (client shorthand)
//! Internally it is checked whether the server answers with PONG. If not, an error is returned.
//! ```
//!# use core::str::FromStr;
//!# use core::net::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//! let mut stack = Stack::default();
//! let clock = StandardClock::default();
//!
//! let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//! let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!
//! let response = client.ping().unwrap().wait().unwrap();
//! ```
//! # Verbose command
//! Sending a `PingCommand` as alternative to client shorthand.
//! ```
//!# use core::str::FromStr;
//!# use core::net::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::ping::PingCommand;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let mut stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//!# let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!#
//! let command = PingCommand::new(None);
//! let response = client.send(command).unwrap().wait().unwrap();
//! ```
//! # Custom argument
//! Optionally, a user-defined argument can be specified.
//!
//! The abstraction compares the server's response with the argument and returns an error if there is no match.
//! ```
//!# use core::str::FromStr;
//!# use core::net::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::ping::PingCommand;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let mut stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//!# let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!#
//! let command = PingCommand::new(Some("hello world".into()));
//! let response = client.send(command).unwrap().wait().unwrap();
//! ```
use crate::commands::auth::AuthCommand;
use crate::commands::builder::{CommandBuilder, ToStringOption};
use crate::commands::hello::HelloCommand;
use crate::commands::{Command, ResponseTypeError};
use crate::network::protocol::Protocol;
use crate::network::{Client, CommandErrors, Future};
use bytes::Bytes;
use embedded_nal::TcpClientStack;
use embedded_time::Clock;

/// Abstraction for PING command
pub struct PingCommand {
    argument: Option<Bytes>,
}

impl PingCommand {
    pub fn new(argument: Option<Bytes>) -> Self {
        PingCommand { argument }
    }
}

static PONG: Bytes = Bytes::from_static(b"PONG");

impl<F> Command<F> for PingCommand
where
    F: From<CommandBuilder> + ToStringOption,
{
    type Response = ();

    fn encode(&self) -> F {
        CommandBuilder::new("PING").arg_option(self.argument.as_ref()).into()
    }

    fn eval_response(&self, frame: F) -> Result<Self::Response, ResponseTypeError> {
        let response = frame.to_string_option().ok_or(ResponseTypeError {})?;
        let pong = &PONG;
        let expected = self.argument.as_ref().unwrap_or(pong);

        if response.as_bytes() != expected.as_ref() {
            return Err(ResponseTypeError {});
        }

        Ok(())
    }
}

impl<'a, N: TcpClientStack, C: Clock, P: Protocol> Client<'a, N, C, P>
where
    AuthCommand: Command<<P as Protocol>::FrameType>,
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    /// Shorthand for [PingCommand]
    pub fn ping(&'a self) -> Result<Future<'a, N, C, P, PingCommand>, CommandErrors>
    where
        <P as Protocol>::FrameType: ToStringOption,
        <P as Protocol>::FrameType: From<CommandBuilder>,
    {
        self.send(PingCommand::new(None))
    }
}
