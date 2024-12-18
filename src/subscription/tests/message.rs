use crate::network::tests::mocks::MockFrames;
use crate::subscription::messages::{DecodeError, Message, ToPushMessage};
use bytes::Bytes;
use redis_protocol::resp2::types::BytesFrame as Resp2Frame;
use redis_protocol::resp3::types::BytesFrame as Resp3Frame;

#[test]
fn test_decode_resp3_no_push() {
    assert_eq!(Message::Unknown, MockFrames::ok_resp3().decode_push().unwrap())
}

#[test]
fn test_decode_resp2_no_push() {
    assert_eq!(Message::Unknown, MockFrames::ok_resp2().decode_push().unwrap())
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
fn test_decode_resp2_incomplete_push() {
    let frame = Resp2Frame::Array(vec![Resp2Frame::SimpleString(Bytes::from_static(b"subscribe"))]);

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
fn test_decode_resp2_invalid_channel_count() {
    let frame = Resp2Frame::Array(vec![
        Resp2Frame::SimpleString(Bytes::from_static(b"subscribe")),
        Resp2Frame::SimpleString(Bytes::from_static(b"test_channel")),
        Resp2Frame::SimpleString(Bytes::from_static(b"not_a_number")),
    ]);

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
fn test_decode_resp2_subscribe_correct() {
    let frame = Resp2Frame::Array(vec![
        Resp2Frame::SimpleString(Bytes::from_static(b"subscribe")),
        Resp2Frame::SimpleString(Bytes::from_static(b"test_channel")),
        Resp2Frame::Integer(4),
    ]);

    assert_eq!(Message::SubConfirmation(4), frame.decode_push().unwrap())
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
fn test_decode_resp2_unsubscribe_correct() {
    let frame = Resp2Frame::Array(vec![
        Resp2Frame::SimpleString(Bytes::from_static(b"unsubscribe")),
        Resp2Frame::SimpleString(Bytes::from_static(b"test_channel")),
        Resp2Frame::Integer(0),
    ]);

    assert_eq!(Message::UnSubConfirmation(0), frame.decode_push().unwrap())
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
fn test_decode_resp2_message_invalid_channel() {
    let frame = Resp2Frame::Array(vec![
        Resp2Frame::SimpleString(Bytes::from_static(b"message")),
        Resp2Frame::Integer(0),
        Resp2Frame::SimpleString(Bytes::from_static(b"payload")),
    ]);

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
fn test_decode_resp2_message_invalid_payload() {
    let frame = Resp2Frame::Array(vec![
        Resp2Frame::SimpleString(Bytes::from_static(b"message")),
        Resp2Frame::SimpleString(Bytes::from_static(b"channel")),
        Resp2Frame::Array(vec![]),
    ]);

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
fn test_decode_resp2_message_simple_string() {
    let frame = Resp2Frame::Array(vec![
        Resp2Frame::SimpleString(Bytes::from_static(b"message")),
        Resp2Frame::SimpleString(Bytes::from_static(b"channel")),
        Resp2Frame::SimpleString(Bytes::from_static(b"payload")),
    ]);

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

#[test]
fn test_decode_resp2_message_blob() {
    let frame = Resp2Frame::Array(vec![
        Resp2Frame::BulkString(Bytes::from_static(b"message")),
        Resp2Frame::BulkString(Bytes::from_static(b"channel")),
        Resp2Frame::BulkString(Bytes::from_static(b"payload")),
    ]);

    assert_eq!(
        Message::Publish(Bytes::from_static(b"channel"), Bytes::from_static(b"payload")),
        frame.decode_push().unwrap()
    )
}
