use crate::network::tests::mocks::MockFrames;
use crate::subscribe::messages::{DecodeError, Message, ToPushMessage};
use bytes::Bytes;
use redis_protocol::resp3::types::Frame as Resp3Frame;

#[test]
fn test_decode_resp3_no_push() {
    assert_eq!(
        DecodeError::NoPushMessage,
        MockFrames::ok_resp3().decode_push().unwrap_err()
    )
}

#[test]
fn test_decode_resp3_incomplete_push() {
    let frame = Resp3Frame::Push {
        data: vec![Resp3Frame::SimpleString {
            data: Bytes::from_static(b"subscribe"),
            attributes: None,
        }],
        attributes: None,
    };

    assert_eq!(DecodeError::ProtocolViolation, frame.decode_push().unwrap_err())
}

#[test]
fn test_decode_resp3_incomplete_pub_sub() {
    let frame = Resp3Frame::Push {
        data: vec![
            Resp3Frame::SimpleString {
                data: Bytes::from_static(b"subscribe"),
                attributes: None,
            },
            Resp3Frame::SimpleString {
                data: Bytes::from_static(b"test"),
                attributes: None,
            },
        ],
        attributes: None,
    };

    assert_eq!(DecodeError::ProtocolViolation, frame.decode_push().unwrap_err())
}

#[test]
fn test_decode_resp3_subscribe_invalid_channel_count() {
    let frame = Resp3Frame::Push {
        data: vec![
            Resp3Frame::SimpleString {
                data: Bytes::from_static(b"subscribe"),
                attributes: None,
            },
            Resp3Frame::SimpleString {
                data: Bytes::from_static(b"test"),
                attributes: None,
            },
            Resp3Frame::SimpleString {
                data: Bytes::from_static(b"count"),
                attributes: None,
            },
        ],
        attributes: None,
    };

    assert_eq!(DecodeError::ProtocolViolation, frame.decode_push().unwrap_err())
}

#[test]
fn test_decode_resp3_subscribe_correct() {
    let frame = Resp3Frame::Push {
        data: vec![
            Resp3Frame::SimpleString {
                data: Bytes::from_static(b"subscribe"),
                attributes: None,
            },
            Resp3Frame::SimpleString {
                data: Bytes::from_static(b"test"),
                attributes: None,
            },
            Resp3Frame::Number {
                data: 3,
                attributes: None,
            },
        ],
        attributes: None,
    };

    assert_eq!(Message::SubConfirmation(3), frame.decode_push().unwrap())
}

#[test]
fn test_decode_resp3_unsubscribe_correct() {
    let frame = Resp3Frame::Push {
        data: vec![
            Resp3Frame::SimpleString {
                data: Bytes::from_static(b"unsubscribe"),
                attributes: None,
            },
            Resp3Frame::SimpleString {
                data: Bytes::from_static(b"channel"),
                attributes: None,
            },
            Resp3Frame::Number {
                data: 6,
                attributes: None,
            },
        ],
        attributes: None,
    };

    assert_eq!(Message::UnSubConfirmation(6), frame.decode_push().unwrap())
}

#[test]
fn test_decode_resp3_message_invalid_channel() {
    let frame = Resp3Frame::Push {
        data: vec![
            Resp3Frame::SimpleString {
                data: Bytes::from_static(b"message"),
                attributes: None,
            },
            Resp3Frame::Number {
                data: 6,
                attributes: None,
            },
            Resp3Frame::SimpleString {
                data: Bytes::from_static(b"payload"),
                attributes: None,
            },
        ],
        attributes: None,
    };

    assert_eq!(DecodeError::ProtocolViolation, frame.decode_push().unwrap_err())
}

#[test]
fn test_decode_resp3_message_invalid_payload() {
    let frame = Resp3Frame::Push {
        data: vec![
            Resp3Frame::SimpleString {
                data: Bytes::from_static(b"message"),
                attributes: None,
            },
            Resp3Frame::SimpleString {
                data: Bytes::from_static(b"channel"),
                attributes: None,
            },
            Resp3Frame::Array {
                data: vec![],
                attributes: None,
            },
        ],
        attributes: None,
    };

    assert_eq!(DecodeError::ProtocolViolation, frame.decode_push().unwrap_err())
}

#[test]
fn test_decode_resp3_message_simple_string() {
    let frame = Resp3Frame::Push {
        data: vec![
            Resp3Frame::SimpleString {
                data: Bytes::from_static(b"message"),
                attributes: None,
            },
            Resp3Frame::SimpleString {
                data: Bytes::from_static(b"channel"),
                attributes: None,
            },
            Resp3Frame::SimpleString {
                data: Bytes::from_static(b"payload"),
                attributes: None,
            },
        ],
        attributes: None,
    };

    assert_eq!(
        Message::Publish(Bytes::from_static(b"channel"), Bytes::from_static(b"payload")),
        frame.decode_push().unwrap()
    )
}

#[test]
fn test_decode_resp3_message_blob() {
    let frame = Resp3Frame::Push {
        data: vec![
            Resp3Frame::SimpleString {
                data: Bytes::from_static(b"message"),
                attributes: None,
            },
            Resp3Frame::BlobString {
                data: Bytes::from_static(b"channel"),
                attributes: None,
            },
            Resp3Frame::BlobString {
                data: Bytes::from_static(b"payload"),
                attributes: None,
            },
        ],
        attributes: None,
    };

    assert_eq!(
        Message::Publish(Bytes::from_static(b"channel"), Bytes::from_static(b"payload")),
        frame.decode_push().unwrap()
    )
}
