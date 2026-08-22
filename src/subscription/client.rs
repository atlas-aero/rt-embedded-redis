use crate::commands::builder::CommandBuilder;
use crate::commands::hello::HelloCommand;
use crate::commands::Command;
use crate::network::protocol::Protocol;
use crate::network::timeout::Timeout;
use crate::network::{Client, CommandErrors};
use crate::subscription::messages::{DecodeError, Message as PushMessage, ToPushMessage};
use bytes::Bytes;
use embedded_io_async::{Read, Write};
use embedded_time::Clock;
use futures_util::future::{select, Either};
use futures_util::pin_mut;

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
    /// Subscription or unsubscription was not confirmed by Redis within the time limit.
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
pub struct Subscription<'a, T: Read + Write, C: Clock, P: Protocol, const L: usize>
where
    HelloCommand: Command<<P as Protocol>::FrameType>,
    <P as Protocol>::FrameType: From<CommandBuilder>,
    <P as Protocol>::FrameType: ToPushMessage,
{
    client: Client<'a, T, C, P>,

    /// List of subscribed topics
    channels: [Bytes; L],

    /// Confirmed + active subscription
    subscribed: bool,
}

impl<'a, T, C, P, const L: usize> Subscription<'a, T, C, P, L>
where
    T: Read + Write,
    C: Clock,
    P: Protocol,
    HelloCommand: Command<<P as Protocol>::FrameType>,
    <P as Protocol>::FrameType: From<CommandBuilder>,
    <P as Protocol>::FrameType: ToPushMessage,
{
    pub fn new(client: Client<'a, T, C, P>, topics: [Bytes; L]) -> Self {
        Self {
            client,
            channels: topics,
            subscribed: false,
        }
    }

    /// Waits for and receives the next published message.
    pub async fn receive(&mut self) -> Result<Option<Message>, Error> {
        loop {
            let message = self.receive_message().await?;

            if message.is_none() {
                continue;
            }

            if let PushMessage::Publish(channel, payload) = message.unwrap() {
                return Ok(Some(Message { channel, payload }));
            }
        }
    }

    /// Starts the subscription and waits for confirmation
    pub(crate) async fn subscribe(mut self) -> Result<Self, Error> {
        let mut cmd = CommandBuilder::new("SUBSCRIBE");
        for topic in &self.channels {
            cmd = cmd.arg(topic);
        }

        self.client.network.send_frame(cmd.into()).await.map_err(Error::CommandError)?;
        self.wait_for_confirmation(|message| message == PushMessage::SubConfirmation(self.channels.len()))
            .await?;

        self.subscribed = true;
        Ok(self)
    }

    /// Unsubscribes from all topics and waits for confirmation
    ///
    /// If this fails, the owned connection is dropped to avoid reusing an undefined state.
    pub async fn unsubscribe(mut self) -> Result<(), Error> {
        self.close().await
    }

    /// Unsubscribes from all topics and waits for confirmation
    pub(crate) async fn close(&mut self) -> Result<(), Error> {
        self.subscribed = false;
        let cmd = CommandBuilder::new("UNSUBSCRIBE");

        self.client.network.send_frame(cmd.into()).await.map_err(Error::CommandError)?;
        self.wait_for_confirmation(|message| message == PushMessage::UnSubConfirmation(0))
            .await?;

        Ok(())
    }

    /// Waits for the confirmation of all topics
    async fn wait_for_confirmation<F: Fn(PushMessage) -> bool>(
        &self,
        is_confirmation: F,
    ) -> Result<(), Error> {
        let timeout =
            Timeout::new(self.client.clock, self.client.timeout_duration).map_err(|_| Error::ClockError)?;

        loop {
            if timeout.is_enabled() {
                let receive = self.receive_message();
                let expired = timeout.wait();
                pin_mut!(receive, expired);

                match select(receive, expired).await {
                    Either::Left((message, _)) => {
                        if let Some(message) = message? {
                            if is_confirmation(message) {
                                return Ok(());
                            }
                        }
                    }
                    Either::Right((result, _)) => {
                        result.map_err(|_| Error::ClockError)?;
                        return Err(Error::Timeout);
                    }
                }
            } else if let Some(message) = self.receive_message().await? {
                if is_confirmation(message) {
                    return Ok(());
                }
            }
        }
    }

    /// Receives and decodes the next message. Returns None in case no message is pending or not complete yet.
    async fn receive_message(&self) -> Result<Option<PushMessage>, Error> {
        let mut frame = self.client.network.take_next_frame();

        if frame.is_none() {
            let received = self.client.network.receive_chunk().await.map_err(|_| Error::TcpError)?;
            if received == 0 {
                return Err(Error::TcpError);
            }
            frame = self.client.network.take_next_frame();
        }

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
