//! Abstraction for arbitrary commands.
//!
//! [CustomCommand] in combination with [CommandBuilder] can be used for executing arbitrary commands,
//! which high level logic is not abstracted yet by this crate.
//!
//! Response is not evaluated, so pure [Resp2Frame](redis_protocol::resp2::types::BytesFrame)
//! or [Resp3Frame](redis_protocol::resp3::types::BytesFrame) is returned.
//! The only exception is that error responses are intercepted and converted to [CommandErrors::ErrorResponse](crate::network::CommandErrors::ErrorResponse)
//!
//! *Please consider contributing command abstractions not supported yet.*
//!
//! # Basic usage
//! The following Example demonstrates execution of [ECHO](<https://redis.io/commands/echo/>) command
//! ```
//!# use core::str::FromStr;
//!# use core::net::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::builder::CommandBuilder;
//!# use embedded_redis::network::ConnectionHandler;
//!# use redis_protocol::resp2::types::Resp2Frame;
//!#
//! let mut stack = Stack::default();
//! let clock = StandardClock::default();
//!
//! let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//! let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!
//! let command = CommandBuilder::new("ECHO").arg_static("Hello World!").to_command();
//! let response = client.send(command).unwrap().wait().unwrap();
//! assert_eq!("Hello World!", response.to_string().unwrap());
//! ```
use crate::commands::builder::CommandBuilder;
use crate::commands::{Command, ResponseTypeError};

/// Abstraction for arbitrary commands.
pub struct CustomCommand {
    builder: CommandBuilder,
}

impl CustomCommand {
    pub fn new(builder: CommandBuilder) -> Self {
        CustomCommand { builder }
    }
}

impl<F> Command<F> for CustomCommand
where
    F: From<CommandBuilder>,
{
    type Response = F;

    fn encode(&self) -> F {
        self.builder.clone().into()
    }

    fn eval_response(&self, frame: F) -> Result<Self::Response, ResponseTypeError> {
        Ok(frame)
    }
}
