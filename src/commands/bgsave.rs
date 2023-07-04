//! Abstraction of BGSAVE command.
//!
//! For general information about this command, see the [Redis documentation](<https://redis.io/commands/bgsave/>).
//!
//! # Basic usage
//! By default no `SCHEDULE` option is used.
//! ```
//!# use core::str::FromStr;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::bgsave::BackgroundSaveCommand;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//! let mut stack = Stack::default();
//! let clock = StandardClock::default();
//!
//! let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//! let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!
//! let command = BackgroundSaveCommand::default();
//! let response = client.send(command).unwrap().wait().unwrap();
//! ```
//! # Schedule option
//! Using `SCHEDULE` option by setting constructor flag to `true`.
//! ```
//!# use core::str::FromStr;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::bgsave::BackgroundSaveCommand;
//!# use embedded_redis::network::ConnectionHandler;
//!#
//!# let mut stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//!# let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!#
//! let command = BackgroundSaveCommand::new(true);
//! ```
//! # Shorthand
//! [Client](Client#method.get) provides a shorthand method for this command.
//! ```no_run
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
//! let response = client.bgsave(false).unwrap().wait().unwrap();
//! ```
use crate::commands::auth::AuthCommand;
use crate::commands::builder::CommandBuilder;
use crate::commands::hello::HelloCommand;
use crate::commands::{Command, ResponseTypeError};
use crate::network::protocol::Protocol;
use crate::network::{Client, CommandErrors, Future};
use bytes::Bytes;
use embedded_nal::TcpClientStack;
use embedded_time::Clock;

static SCHEDULE_OPTION: Bytes = Bytes::from_static(b"SCHEDULE");

/// Abstraction ob BGSAVE command
#[derive(Default)]
pub struct BackgroundSaveCommand {
    /// If BGSAVE SCHEDULE is used, the command will immediately return OK when an AOF rewrite is
    /// in progress and schedule the background save to run at the next opportunity.
    schedule: bool,
}

impl BackgroundSaveCommand {
    pub fn new(schedule: bool) -> Self {
        Self { schedule }
    }
}

impl<F: From<CommandBuilder>> Command<F> for BackgroundSaveCommand {
    type Response = ();

    fn encode(&self) -> F {
        let builder = CommandBuilder::new("BGSAVE");

        if self.schedule {
            return builder.arg(&SCHEDULE_OPTION).into();
        }

        builder.into()
    }

    fn eval_response(&self, _: F) -> Result<Self::Response, ResponseTypeError> {
        Ok(())
    }
}

impl<'a, N: TcpClientStack, C: Clock, P: Protocol> Client<'a, N, C, P>
where
    AuthCommand: Command<<P as Protocol>::FrameType>,
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    /// Shorthand for [BackgroundSaveCommand]
    pub fn bgsave(
        &'a self,
        schedule: bool,
    ) -> Result<Future<'a, N, C, P, BackgroundSaveCommand>, CommandErrors>
    where
        <P as Protocol>::FrameType: From<CommandBuilder>,
    {
        self.send(BackgroundSaveCommand::new(schedule))
    }
}
