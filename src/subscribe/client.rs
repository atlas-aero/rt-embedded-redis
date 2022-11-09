use crate::commands::builder::CommandBuilder;
use crate::commands::hello::HelloCommand;
use crate::commands::Command;
use crate::network::protocol::Protocol;
use crate::network::timeout::Timeout;
use crate::network::{Client, CommandErrors};
use crate::subscribe::messages::{DecodeError, Message, ToPushMessage};
use bytes::Bytes;
use embedded_nal::TcpClientStack;
use embedded_time::Clock;

/// Subscription errors
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Error {
    /// Error while sending SUBSCRIBE command
    CommandError(CommandErrors),
    /// Upstream time error
    ClockError,
    /// Network error receiving data
    TcpError,
    /// Error while decoding a push message. Either Redis sent invalid data or there is a decoder bug.
    DecodeError,
    /// Subscription was not confirmed by Redis within time limit. Its recommended to close/reconnect the socket to avoid
    /// subsequent errors based on invalid state.
    Timeout,
}

/// Client for handling subscriptions
/// L: Number of subscribed topics
#[derive(Debug)]
pub struct Subscription<'a, N: TcpClientStack, C: Clock, P: Protocol, const L: usize>
where
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    client: Client<'a, N, C, P>,

    /// List of subscribed topics
    channels: [Bytes; L],
}

impl<'a, N, C, P, const L: usize> Subscription<'a, N, C, P, L>
where
    N: TcpClientStack,
    C: Clock,
    P: Protocol,
    HelloCommand: Command<<P as Protocol>::FrameType>,
    <P as Protocol>::FrameType: From<CommandBuilder>,
    <P as Protocol>::FrameType: ToPushMessage,
{
    pub fn new(client: Client<'a, N, C, P>, topics: [Bytes; L]) -> Self {
        Self {
            client,
            channels: topics,
        }
    }

    /// Starts the subscription and waits for confirmation
    pub(crate) fn subscribe(self) -> Result<Self, Error> {
        let mut cmd = CommandBuilder::new("SUBSCRIBE");
        for topic in &self.channels {
            cmd = cmd.arg(topic);
        }

        self.client.network.send_frame(cmd.into()).map_err(Error::CommandError)?;
        self.wait_for_confirmation()
    }

    /// Waits for the confirmation of all topics
    fn wait_for_confirmation(self) -> Result<Self, Error> {
        let timeout =
            Timeout::new(self.client.clock, self.client.timeout_duration).map_err(|_| Error::ClockError)?;

        while !timeout.expired().map_err(|_| Error::ClockError)? {
            if let Some(Message::SubConfirmation(count)) = self.receive_message()? {
                if count == self.channels.len() {
                    return Ok(self);
                }
            }
        }

        Err(Error::Timeout)
    }

    /// Receives and decodes the next message. Returns None in case no message is pending or not complete yet.
    fn receive_message(&self) -> Result<Option<Message>, Error> {
        if let Err(error) = self.client.network.receive_chunk() {
            return match error {
                nb::Error::Other(_) => Err(Error::TcpError),
                nb::Error::WouldBlock => Ok(None),
            };
        }

        let frame = self.client.network.take_next_frame();
        if frame.is_none() {
            return Ok(None);
        }

        match frame.unwrap().decode_push() {
            Ok(message) => Ok(Some(message)),
            Err(error) => match error {
                DecodeError::NoPushMessage => Ok(None),
                DecodeError::UnknownType => Ok(None),
                DecodeError::ProtocolViolation => Err(Error::DecodeError),
                DecodeError::IntegerOverflow => Err(Error::DecodeError),
            },
        }
    }
}
