use bytes::Bytes;
use redis_protocol::resp3::prelude::{Frame as Resp3Frame, Frame};

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
    fn decode_push(self) -> Result<Message, DecodeError>;
}

impl ToPushMessage for Resp3Frame {
    fn decode_push(self) -> Result<Message, DecodeError> {
        Resp3Decoder::new(self).decode()
    }
}

struct Resp3Decoder {
    frame: Resp3Frame,
}

impl Resp3Decoder {
    pub fn new(frame: Resp3Frame) -> Self {
        Self { frame }
    }

    pub fn decode(self) -> Result<Message, DecodeError> {
        match &self.frame {
            Frame::Push { data, attributes: _ } => {
                if data.len() < 2 {
                    return Err(DecodeError::ProtocolViolation);
                }

                if data.len() < 3 {
                    return Err(DecodeError::ProtocolViolation);
                }

                match self.get_byte_string(&data[0])? {
                    b"message" => self.decode_message(data),
                    b"subscribe" => self.decode_subscribe(data),
                    b"unsubscribe" => self.decode_unsubscribe(data),
                    &_ => Err(DecodeError::UnknownType),
                }
            }
            _ => Err(DecodeError::NoPushMessage),
        }
    }

    /// Decodes and validates a "subscribe" message
    fn decode_subscribe(&self, data: &[Frame]) -> Result<Message, DecodeError> {
        match &data[2] {
            Frame::Number { data, attributes: _ } => {
                Ok(Message::SubConfirmation(self.cast_channel_count(*data)?))
            }
            _ => Err(DecodeError::ProtocolViolation),
        }
    }

    /// Decodes and validates a "unsubscribe" message
    fn decode_unsubscribe(&self, data: &[Frame]) -> Result<Message, DecodeError> {
        match &data[2] {
            Frame::Number { data, attributes: _ } => {
                Ok(Message::UnSubConfirmation(self.cast_channel_count(*data)?))
            }
            _ => Err(DecodeError::ProtocolViolation),
        }
    }

    /// Decodes and validates a "message" message
    fn decode_message(&self, data: &[Frame]) -> Result<Message, DecodeError> {
        Ok(Message::Publish(
            self.clone_string(&data[1])?,
            self.clone_string(&data[2])?,
        ))
    }

    /// Tries to convert the frame to a byte string
    fn get_byte_string<'a>(&self, frame: &'a Resp3Frame) -> Result<&'a [u8], DecodeError> {
        let byte_string = match frame {
            Frame::BlobString { data, attributes: _ } | Frame::SimpleString { data, attributes: _ } => data,
            _ => return Err(DecodeError::ProtocolViolation),
        };

        Ok(&byte_string[..])
    }

    /// Safe casting of channel count
    fn cast_channel_count(&self, count: i64) -> Result<usize, DecodeError> {
        if count.is_negative() {
            return Err(DecodeError::ProtocolViolation);
        }

        usize::try_from(count).map_err(|_| DecodeError::IntegerOverflow)
    }

    /// Tries to extract and clone the Bytes string
    fn clone_string(&self, frame: &Resp3Frame) -> Result<Bytes, DecodeError> {
        match frame {
            Frame::BlobString { data, attributes: _ } | Frame::SimpleString { data, attributes: _ } => {
                Ok(data.clone())
            }
            _ => Err(DecodeError::ProtocolViolation),
        }
    }
}
