use bytes::Bytes;
use redis_protocol::resp3::prelude::Frame as Resp3Frame;

/// A decoded PubSub message
#[derive(Debug, PartialEq, Eq)]
pub enum Message {
    /// Confirmation of a subscription. The integer represents the number of channels currently subscribed to.
    SubConfirmation(usize),
    /// Confirmation of a unsubscribe command. The integer represents the number of channels currently subscribed to.
    UnSubConfirmation(usize),
    /// An actual published message. First value represents the channel, the second value is the actual message payload.
    Publish(Bytes, Bytes),
}

/// Errors related for decoding push messages
#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The given frame is not a push message (only RESP3)
    NoPushMessage,
    /// Unknown sub message type (neither subscribe, unsubscribe nor message)
    UnknownType,
    /// Invalid message format (violation of RESP2 or RESP3 specification)
    ProtocolViolation,
    /// The given channel counts overflows usize
    IntegerOverflow,
}

/// Decodes frames to messages
pub trait ToPushMessage {
    /// Tries to decode the frame to a push message
    fn decode_push(self) -> Result<Message, DecodeError>
    where
        Self: Sized,
    {
        Decoder::new(self).decode()
    }

    /// Validates that the given frame is a push message and returns the inner array.
    fn as_array(&self) -> Result<&[Self], DecodeError>
    where
        Self: Sized;

    /// Validates that the given frame is a string type and clones the inner Bytes value.
    fn clone_byte_string(&self, frame: &Self) -> Result<Bytes, DecodeError>;

    /// Validates that the given frame is a number type and returns the inner value.
    fn get_number(&self, frame: &Self) -> Result<i64, DecodeError>;
}

impl ToPushMessage for Resp3Frame {
    fn as_array(&self) -> Result<&[Self], DecodeError> {
        if let Resp3Frame::Push { data, attributes: _ } = self {
            return Ok(data);
        }

        Err(DecodeError::NoPushMessage)
    }

    fn clone_byte_string(&self, frame: &Self) -> Result<Bytes, DecodeError> {
        match frame {
            Resp3Frame::BlobString { data, attributes: _ }
            | Resp3Frame::SimpleString { data, attributes: _ } => Ok(data.clone()),
            _ => Err(DecodeError::ProtocolViolation),
        }
    }

    fn get_number(&self, frame: &Self) -> Result<i64, DecodeError> {
        if let Resp3Frame::Number { data, attributes: _ } = frame {
            return Ok(*data);
        }

        Err(DecodeError::ProtocolViolation)
    }
}

/// Generic push message decoder for RESP2 + RESP3 frames
struct Decoder<F: ToPushMessage> {
    frame: F,
}

impl<F: ToPushMessage> Decoder<F> {
    pub fn new(frame: F) -> Self {
        Self { frame }
    }

    pub fn decode(self) -> Result<Message, DecodeError> {
        let data = self.frame.as_array()?;

        if data.len() < 3 {
            return Err(DecodeError::ProtocolViolation);
        }

        match &self.frame.clone_byte_string(&data[0])?[..] {
            b"message" => self.decode_message(data),
            b"subscribe" => self.decode_subscribe(data),
            b"unsubscribe" => self.decode_unsubscribe(data),
            &_ => Err(DecodeError::UnknownType),
        }
    }

    /// Decodes and validates a "subscribe" message
    fn decode_subscribe(&self, data: &[F]) -> Result<Message, DecodeError> {
        let channel_count = self.frame.get_number(&data[2])?;
        Ok(Message::SubConfirmation(self.cast_channel_count(channel_count)?))
    }

    /// Decodes and validates a "unsubscribe" message
    fn decode_unsubscribe(&self, data: &[F]) -> Result<Message, DecodeError> {
        let channel_count = self.frame.get_number(&data[2])?;
        Ok(Message::UnSubConfirmation(
            self.cast_channel_count(channel_count)?,
        ))
    }

    /// Decodes and validates a "message" message
    fn decode_message(&self, data: &[F]) -> Result<Message, DecodeError> {
        Ok(Message::Publish(
            self.frame.clone_byte_string(&data[1])?,
            self.frame.clone_byte_string(&data[2])?,
        ))
    }

    /// Safe casting of channel count
    fn cast_channel_count(&self, count: i64) -> Result<usize, DecodeError> {
        if count.is_negative() {
            return Err(DecodeError::ProtocolViolation);
        }

        usize::try_from(count).map_err(|_| DecodeError::IntegerOverflow)
    }
}
