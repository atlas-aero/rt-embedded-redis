use crate::network::client::CommandErrors;
use alloc::string::ToString;
use bytes::{Bytes, BytesMut};
use redis_protocol::resp2::types::Frame as Resp2Frame;
use redis_protocol::resp3::prelude::Frame;
use redis_protocol::resp3::types::DecodedFrame;
use redis_protocol::resp3::types::Frame as Resp3Frame;
use redis_protocol::types::RedisProtocolError;
use redis_protocol::{resp2, resp3};

/// Generic wrapper for redis-protocol encode/decode methods
pub trait Protocol: Clone {
    type FrameType;

    /// Decodes bytes to Frame
    fn decode(&self, data: &Bytes) -> Result<Option<(Self::FrameType, usize)>, RedisProtocolError>;

    /// Encodes Frame to buffer
    fn encode_bytes(&self, buf: &mut BytesMut, frame: &Self::FrameType) -> Result<usize, RedisProtocolError>;

    /// Wraps response error to CommandError
    fn assert_error(&self, frame: &Self::FrameType) -> Result<(), CommandErrors>;

    /// Returns true if protocol switch, respectively HELLO command, is needed
    fn requires_hello(&self) -> bool {
        false
    }
}

/// Abstraction for RESP2 protocol
#[derive(Clone, Debug)]
pub struct Resp2 {}

impl Protocol for Resp2 {
    type FrameType = Resp2Frame;

    fn decode(&self, data: &Bytes) -> Result<Option<(Self::FrameType, usize)>, RedisProtocolError> {
        resp2::decode::decode(data)
    }

    fn encode_bytes(&self, buf: &mut BytesMut, frame: &Self::FrameType) -> Result<usize, RedisProtocolError> {
        resp2::encode::encode_bytes(buf, frame)
    }

    fn assert_error(&self, frame: &Self::FrameType) -> Result<(), CommandErrors> {
        match frame {
            Resp2Frame::Error(message) => Err(CommandErrors::ErrorResponse(message.to_string())),
            _ => Ok(()),
        }
    }
}

/// Abstraction for RESP3 protocol
#[derive(Clone, Debug)]
pub struct Resp3 {}

impl Protocol for Resp3 {
    type FrameType = Resp3Frame;

    /// Currently just decodes complete frames
    /// In case of streaming frame None is returned. In Redis <= 6 there is currently no command
    /// returning a stream frame.
    fn decode(&self, data: &Bytes) -> Result<Option<(Self::FrameType, usize)>, RedisProtocolError> {
        match resp3::decode::streaming::decode(data) {
            Ok(option) => match option {
                None => Ok(None),
                Some(decoded) => {
                    let (frame, size) = decoded;
                    match frame {
                        DecodedFrame::Streaming(_) => Ok(None),
                        DecodedFrame::Complete(complete_frame) => Ok(Some((complete_frame, size))),
                    }
                }
            },
            Err(error) => Err(error),
        }
    }

    fn encode_bytes(&self, buf: &mut BytesMut, frame: &Self::FrameType) -> Result<usize, RedisProtocolError> {
        resp3::encode::complete::encode_bytes(buf, frame)
    }

    fn assert_error(&self, frame: &Self::FrameType) -> Result<(), CommandErrors> {
        match frame {
            Frame::BlobError { .. } => Err(CommandErrors::ErrorResponse("blob".to_string())),
            Frame::SimpleError { data, attributes: _ } => Err(CommandErrors::ErrorResponse(data.to_string())),
            _ => Ok(()),
        }
    }

    fn requires_hello(&self) -> bool {
        true
    }
}
