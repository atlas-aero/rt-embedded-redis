use crate::commands::Command;
use crate::network::buffer::Network;
use crate::network::client::CommandErrors;
use crate::network::client::CommandErrors::CommandResponseViolation;
use crate::network::protocol::Protocol;
use crate::network::timeout::Timeout;
use embedded_io_async::{Read, Write};
use embedded_time::Clock;
use futures_util::future::{select, Either};
use futures_util::pin_mut;

#[derive(Clone)]
pub(crate) struct Identity {
    /// Used for invalidating futures
    /// Gets incremented on fatal problems like timeouts or fault responses, on which message<->future
    /// mapping can no longer be guaranteed
    pub series: usize,

    /// Unique index of mapping future to response message
    pub index: usize,
}

/// Asynchronous response management for a command sent to Redis.
pub(crate) struct Future<'a, T: Read + Write, C: Clock, P: Protocol, Cmd: Command<P::FrameType>> {
    id: Identity,
    command: Cmd,
    protocol: P,
    network: &'a Network<T, P>,
    timeout: Timeout<'a, C>,

    /// Was wait called? Flag is used for destructor.
    wait_called: bool,
}

impl<'a, T: Read + Write, C: Clock, P: Protocol, Cmd: Command<P::FrameType>> Future<'a, T, C, P, Cmd> {
    pub(crate) fn new(
        id: Identity,
        command: Cmd,
        protocol: P,
        network: &'a Network<T, P>,
        timeout: Timeout<'a, C>,
    ) -> Future<'a, T, C, P, Cmd> {
        Self {
            id,
            command,
            protocol,
            network,
            timeout,
            wait_called: false,
        }
    }

    /// Waits until the response is received and returns it
    /// Returns an error for an invalid response or timeout (if configured).
    pub(crate) async fn wait(mut self) -> Result<Cmd::Response, CommandErrors> {
        self.wait_called = true;

        self.process().await?;

        // Previous process call ensures that frame is existing
        let frame = self.network.take_frame(&self.id).unwrap();
        self.protocol.assert_error(&frame)?;

        match self.command.eval_response(frame) {
            Ok(response) => Ok(response),
            Err(_) => Err(CommandResponseViolation),
        }
    }

    /// Processes socket data until this response is complete
    async fn process(&mut self) -> Result<(), CommandErrors> {
        while !self.network.is_complete(&self.id)? {
            if self.timeout.is_enabled() {
                let receive = self.network.receive_chunk();
                let timeout = self.timeout.wait();
                pin_mut!(receive, timeout);

                match select(receive, timeout).await {
                    Either::Left((result, _)) => {
                        if result.map_err(|_| CommandErrors::TcpError)? == 0 {
                            return Err(CommandErrors::TcpError);
                        }
                    }
                    Either::Right((result, _)) => {
                        self.network.invalidate_futures();
                        result?;
                        return Err(CommandErrors::Timeout);
                    }
                }
            } else {
                if self.network.receive_chunk().await.map_err(|_| CommandErrors::TcpError)? == 0 {
                    return Err(CommandErrors::TcpError);
                }
            }

            if self.network.is_buffer_full() {
                return Err(CommandErrors::MemoryFull);
            }
        }

        Ok(())
    }
}

impl<T: Read + Write, C: Clock, P: Protocol, Cmd: Command<P::FrameType>> Drop for Future<'_, T, C, P, Cmd> {
    fn drop(&mut self) {
        if !self.wait_called {
            self.network.drop_future(self.id.clone());
        }
    }
}
