use crate::commands::Command;
use crate::network::buffer::Network;
use crate::network::client::CommandErrors;
use crate::network::client::CommandErrors::CommandResponseViolation;
use crate::network::protocol::Protocol;
use crate::network::timeout::Timeout;
use embedded_nal::TcpClientStack;
use embedded_time::Clock;
use nb;

#[derive(Clone)]
pub(crate) struct Identity {
    /// Used for invalidating futures
    /// Gets incremented on fatal problems like timeouts or fault responses, on which message<->future
    /// mapping can no longer be guaranteed
    pub series: usize,

    /// Unique index of mapping future to response message
    pub index: usize,
}

/// Non-blocking response management
pub struct Future<'a, N: TcpClientStack, C: Clock, P: Protocol, Cmd: Command<P::FrameType>> {
    id: Identity,
    command: Cmd,
    protocol: P,
    network: &'a Network<'a, N, P>,
    timeout: Timeout<'a, C>,

    /// Cached error during work of ready(). Will be returned on wait() call.
    error: Option<CommandErrors>,

    /// Was wait called? Flag is used for destructor.
    wait_called: bool,
}

impl<'a, N: TcpClientStack, C: Clock, P: Protocol, Cmd: Command<P::FrameType>> Future<'a, N, C, P, Cmd> {
    pub(crate) fn new(
        id: Identity,
        command: Cmd,
        protocol: P,
        network: &'a Network<'a, N, P>,
        timeout: Timeout<'a, C>,
    ) -> Future<'a, N, C, P, Cmd> {
        Self {
            id,
            command,
            protocol,
            network,
            timeout,
            error: None,
            wait_called: false,
        }
    }

    /// Blocks until response is received and returns the response
    /// Throws an error on invalid response or timeout (if configured)
    pub fn wait(mut self) -> Result<Cmd::Response, CommandErrors> {
        self.wait_called = true;

        if self.error.is_some() {
            return Err(self.error.clone().unwrap());
        }

        self.process(true)?;

        // Previous process call ensures that frame is existing
        let frame = self.network.take_frame(&self.id).unwrap();
        self.protocol.assert_error(&frame)?;

        match self.command.eval_response(frame) {
            Ok(response) => Ok(response),
            Err(_) => Err(CommandResponseViolation),
        }
    }

    /// Non blocking method for checking if data is ready
    /// So if true is returned, wait() is non-blocking
    /// Reads all pending data and returns true if response is ready
    /// Errors are preserved and returned on wait() call
    pub fn ready(&mut self) -> bool {
        match self.process(false) {
            Ok(_) => match self.network.is_complete(&self.id) {
                Ok(result) => result,
                Err(error) => {
                    self.error = Some(error);
                    true
                }
            },
            Err(error) => {
                self.error = Some(error);
                true
            }
        }
    }

    /// Processes socket data
    /// If block=false, only pending data is read without blocking
    fn process(&mut self, block: bool) -> Result<(), CommandErrors> {
        while !self.network.is_complete(&self.id)? {
            let result = self.network.receive_chunk();

            if self.network.is_buffer_full() {
                return Err(CommandErrors::MemoryFull);
            }

            if let Err(error) = result {
                match error {
                    nb::Error::Other(_) => {
                        return Err(CommandErrors::TcpError);
                    }
                    nb::Error::WouldBlock => {
                        if self.timeout.expired()? {
                            self.network.invalidate_futures();
                            return Err(CommandErrors::Timeout);
                        }

                        if !block {
                            return Ok(());
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl<N: TcpClientStack, C: Clock, P: Protocol, Cmd: Command<P::FrameType>> Drop for Future<'_, N, C, P, Cmd> {
    fn drop(&mut self) {
        if !self.wait_called {
            self.network.drop_future(self.id.clone());
        }
    }
}
