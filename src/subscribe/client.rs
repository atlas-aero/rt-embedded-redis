use crate::commands::builder::CommandBuilder;
use crate::commands::hello::HelloCommand;
use crate::commands::Command;
use crate::network::protocol::Protocol;
use crate::network::timeout::Timeout;
use crate::network::{Client, CommandErrors};
use crate::subscribe::messages::{DecodeError, Message as PushMessage, ToPushMessage};
use bytes::Bytes;
use embedded_nal::TcpClientStack;
use embedded_time::Clock;

/// Subscription errors
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Error {
    /// Error while sending SUBSCRIBE or UNSUBSCRIBE command
    CommandError(CommandErrors),
    /// Upstream time error
    ClockError,
    /// Network error receiving or sending data
    TcpError,
    /// Error while decoding a push message. Either Redis sent invalid data or there is a decoder bug.
    DecodeError,
    /// Subscription or Unsubscription was not confirmed by Redis within time limit. Its recommended to close/reconnect the socket to avoid
    /// subsequent errors based on invalid state.
    Timeout,
}

/// A published subscription message
#[derive(Debug, Clone)]
pub struct Message {
    /// The channel the message has been published to
    pub channel: Bytes,

    /// The actual payload
    pub payload: Bytes,
}

/// Client for handling subscriptions
///
/// L: Number of subscribed topics
#[derive(Debug)]
pub struct Subscription<'a, N: TcpClientStack, C: Clock, P: Protocol, const L: usize>
where
    HelloCommand: Command<<P as Protocol>::FrameType>,
    <P as Protocol>::FrameType: From<CommandBuilder>,
    <P as Protocol>::FrameType: ToPushMessage,
{
    client: Client<'a, N, C, P>,

    /// List of subscribed topics
    channels: [Bytes; L],

    /// Confirmed + active subscription
    subscribed: bool,
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
            subscribed: false,
        }
    }

    /// Receives a message. Returns None in case no message is pending
    pub fn receive(&mut self) -> Result<Option<Message>, Error> {
        loop {
            let message = self.receive_message()?;

            if message.is_none() {
                return Ok(None);
            }

            if let PushMessage::Publish(channel, payload) = message.unwrap() {
                return Ok(Some(Message { channel, payload }));
            }
        }
    }

    /// Starts the subscription and waits for confirmation
    pub(crate) fn subscribe(mut self) -> Result<Self, Error> {
        let mut cmd = CommandBuilder::new("SUBSCRIBE");
        for topic in &self.channels {
            cmd = cmd.arg(topic);
        }

        self.client.network.send_frame(cmd.into()).map_err(Error::CommandError)?;
        self.wait_for_confirmation(|message| message == PushMessage::SubConfirmation(self.channels.len()))?;

        self.subscribed = true;
        Ok(self)
    }

    /// Unsubscribes from all topics and waits for confirmation
    ///
    /// *If this fails, it's recommended to clos the connection to avoid subsequent errors caused by invalid state*
    pub fn unsubscribe(mut self) -> Result<(), Error> {
        self.close()
    }

    /// Unsubscribes from all topics and waits for confirmation
    pub(crate) fn close(&mut self) -> Result<(), Error> {
        self.subscribed = false;
        let cmd = CommandBuilder::new("UNSUBSCRIBE");

        self.client.network.send_frame(cmd.into()).map_err(Error::CommandError)?;
        self.wait_for_confirmation(|message| message == PushMessage::UnSubConfirmation(0))?;

        Ok(())
    }

    /// Waits for the confirmation of all topics
    fn wait_for_confirmation<F: Fn(PushMessage) -> bool>(&self, is_confirmation: F) -> Result<(), Error> {
        let timeout =
            Timeout::new(self.client.clock, self.client.timeout_duration).map_err(|_| Error::ClockError)?;

        while !timeout.expired().map_err(|_| Error::ClockError)? {
            if let Some(message) = self.receive_message()? {
                if is_confirmation(message) {
                    return Ok(());
                }
            }
        }

        Err(Error::Timeout)
    }

    /// Receives and decodes the next message. Returns None in case no message is pending or not complete yet.
    fn receive_message(&self) -> Result<Option<PushMessage>, Error> {
        // Receive all pending data
        loop {
            if let Err(error) = self.client.network.receive_chunk() {
                match error {
                    nb::Error::Other(_) => return Err(Error::TcpError),
                    nb::Error::WouldBlock => break,
                };
            }
        }

        let frame = self.client.network.take_next_frame();
        if frame.is_none() {
            return Ok(None);
        }

        match frame.unwrap().decode_push() {
            Ok(message) => Ok(Some(message)),
            Err(error) => match error {
                DecodeError::ProtocolViolation => Err(Error::DecodeError),
                DecodeError::IntegerOverflow => Err(Error::DecodeError),
            },
        }
    }

    /// Prevents the automatic unsubscription when client is dropped
    #[cfg(test)]
    pub(crate) fn set_unsubscribed(&mut self) {
        self.subscribed = false;
    }
}

impl<N, C, P, const L: usize> Drop for Subscription<'_, N, C, P, L>
where
    N: TcpClientStack,
    C: Clock,
    P: Protocol,
    HelloCommand: Command<<P as Protocol>::FrameType>,
    <P as Protocol>::FrameType: From<CommandBuilder>,
    <P as Protocol>::FrameType: ToPushMessage,
{
    fn drop(&mut self) {
        if self.subscribed {
            let _ = self.close();
        }
    }
}
