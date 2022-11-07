use crate::commands::auth::AuthCommand;
use crate::commands::hello::{HelloCommand, HelloResponse};
use crate::commands::Command;
use crate::network::buffer::Network;
use crate::network::future::Future;
use crate::network::handler::{ConnectionError, Credentials};
use crate::network::protocol::{Protocol, Resp3};
use crate::network::timeout::{Timeout, TimeoutError};
use alloc::string::String;
use core::fmt::{Debug, Formatter};
use embedded_nal::TcpClientStack;
use embedded_time::duration::Microseconds;
use embedded_time::Clock;

/// Error handling for command execution
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum CommandErrors {
    /// No response within expected time frame
    /// *Is recommended to create a new client/connection in this case*.
    Timeout,
    /// Failed encoding TX command
    EncodingCommandFailed,
    /// Received an invalid response violating the RESP protocol. Ideally this should never happen.
    /// The following causes are conceivable
    /// * Bug in this library (e.g. incomplete implementation of RESP protocol)
    /// * Redis server bug
    /// * Network failure. As we are using TCP, only a network stack bug or other exotic causes (e.g. bit flip) is reasonable.
    /// *Is recommended to create a new client/connection in this case*.
    ProtocolViolation,
    /// Future is no longer valid. This happens on fatal problems like timeouts or faulty responses, on which message<->future
    /// mapping can no longer be guaranteed
    /// *Is recommended to create a new client/connection in this case*.
    InvalidFuture,
    /// Low level network error
    TcpError,
    /// Upstream timer/clock failure
    TimerError,
    /// Received an unexpected response incompatible with the command specification
    CommandResponseViolation,
    /// Redis error response. Inner value is the error message received.
    ErrorResponse(String),
    /// Response buffer overflow. Either the received/unparsed data did not fit in the buffer of the max. number of futures has been exceeded.
    BufferOverflow,
}

/// Client to execute Redis commands
///
/// The functionality of the client is best explained by a [command example](crate::commands::get).
/// F_COUNT: Max. number of parallel futures
pub struct Client<'a, N: TcpClientStack, C: Clock, P: Protocol, const F_COUNT: usize>
where
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    pub(crate) network: Network<'a, N, P, F_COUNT>,
    pub(crate) clock: Option<&'a C>,

    /// Max. time waiting for response
    pub(crate) timeout_duration: Microseconds,

    /// Response to HELLO command, only used for RESP3
    pub(crate) hello_response: Option<&'a <HelloCommand as Command<<P as Protocol>::FrameType>>::Response>,
}

impl<'a, N: TcpClientStack, C: Clock, P: Protocol, const F_COUNT: usize> Client<'a, N, C, P, F_COUNT>
where
    AuthCommand: Command<<P as Protocol>::FrameType>,
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    /// Sends the given command non-blocking
    pub fn send<Cmd>(&'a self, command: Cmd) -> Result<Future<N, C, P, Cmd, F_COUNT>, CommandErrors>
    where
        Cmd: Command<P::FrameType>,
    {
        let id = self.network.send(command.encode())?;

        Ok(Future::new(
            id,
            command,
            self.network.get_protocol(),
            &self.network,
            Timeout::new(self.clock, self.timeout_duration)?,
        ))
    }

    /// Authenticates blocking with the given credentials during client initialization
    pub(crate) fn auth(&'a self, credentials: Option<Credentials>) -> Result<(), ConnectionError> {
        if credentials.is_some() {
            self.send(AuthCommand::from(credentials.as_ref().unwrap()))
                .map_err(auth_error)?
                .wait()
                .map_err(auth_error)?;
        }

        Ok(())
    }

    /// Prepares the new RESP3 client by authenticating and switching protocol (HELLO command) if needed
    pub(crate) fn init(
        &'a self,
        credentials: Option<Credentials>,
    ) -> Result<Option<<HelloCommand as Command<<P as Protocol>::FrameType>>::Response>, ConnectionError>
    {
        self.auth(credentials)?;
        if self.network.get_protocol().requires_hello() {
            return Ok(Some(
                self.send(HelloCommand {}).map_err(hello_error)?.wait().map_err(hello_error)?,
            ));
        }

        Ok(None)
    }

    /// Waiting on any dropped futures to leave a clean state
    pub fn close(&self) {
        if !self.network.remaining_dropped_futures() {
            return;
        }

        let timer = match Timeout::new(self.clock, self.timeout_duration) {
            Ok(timer) => timer,
            Err(_) => {
                return;
            }
        };

        while self.network.remaining_dropped_futures() && !timer.expired().unwrap_or(true) {
            self.network.handle_dropped_futures();
        }
    }
}

impl<'a, N: TcpClientStack, C: Clock, const F_COUNT: usize> Client<'a, N, C, Resp3, F_COUNT> {
    /// Returns the response to HELLO command executed during connection initialization
    /// [Client HELLO response]
    pub fn get_hello_response(&self) -> &HelloResponse {
        self.hello_response.as_ref().unwrap()
    }
}

impl From<TimeoutError> for CommandErrors {
    fn from(_: TimeoutError) -> Self {
        CommandErrors::TimerError
    }
}

fn auth_error(error: CommandErrors) -> ConnectionError {
    ConnectionError::AuthenticationError(error)
}

#[allow(dead_code)]
fn hello_error(error: CommandErrors) -> ConnectionError {
    ConnectionError::ProtocolSwitchError(error)
}

impl<'a, N: TcpClientStack, C: Clock, P: Protocol, const F_COUNT: usize> Debug
    for Client<'a, N, C, P, F_COUNT>
where
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Client")
            .field("network", &self.network)
            .field("timeout_duration", &self.timeout_duration)
            .finish()
    }
}
