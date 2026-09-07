use crate::commands::helpers::CmdStr;
use crate::commands::keys::KeysCommand;
use crate::commands::Command;
use alloc::vec;
use bytes::Bytes;
use redis_protocol::resp2::types::BytesFrame as Resp2Frame;
use redis_protocol::resp3::types::{BytesFrame as Resp3Frame, FrameMap};

#[test]
fn test_encode_resp2() {
    for pattern in ["*", "user:*", "user:??", "[a-z]*", r"literal\*", ""] {
        let frame: Resp2Frame = KeysCommand::new(pattern).encode();
        assert_eq!(
            Resp2Frame::Array(vec![
                CmdStr::new("KEYS").to_bulk(),
                CmdStr::new(pattern).to_bulk()
            ]),
            frame
        );
    }
}

#[test]
fn test_encode_resp3() {
    for pattern in ["*", "user:*", "user:??", "[a-z]*", r"literal\*", ""] {
        let frame: Resp3Frame = KeysCommand::new(pattern).encode();
        assert_eq!(
            Resp3Frame::Array {
                data: vec![CmdStr::new("KEYS").to_blob(), CmdStr::new(pattern).to_blob()],
                attributes: None,
            },
            frame
        );
    }
}

#[test]
fn test_encode_binary_pattern() {
    let pattern = Bytes::from_static(b"\xff\0*");
    let command = KeysCommand::new(pattern.clone());
    let resp2: Resp2Frame = command.encode();
    let resp3: Resp3Frame = command.encode();

    assert_eq!(
        Resp2Frame::Array(vec![
            CmdStr::new("KEYS").to_bulk(),
            Resp2Frame::BulkString(pattern.clone())
        ]),
        resp2
    );
    assert_eq!(
        Resp3Frame::Array {
            data: vec![
                CmdStr::new("KEYS").to_blob(),
                Resp3Frame::BlobString {
                    data: pattern,
                    attributes: None
                }
            ],
            attributes: None,
        },
        resp3
    );
}

#[test]
fn test_eval_response_resp2_success() {
    let keys = vec![
        Bytes::from_static(b"user:2"),
        Bytes::from_static(b"user:1"),
        Bytes::from_static(b"\xff\0"),
        Bytes::new(),
    ];
    let frame = Resp2Frame::Array(keys.iter().cloned().map(Resp2Frame::BulkString).collect());

    assert_eq!(keys, KeysCommand::new("*").eval_response(frame).unwrap());
}

#[test]
fn test_eval_response_resp3_success() {
    let keys = vec![
        Bytes::from_static(b"user:2"),
        Bytes::from_static(b"user:1"),
        Bytes::from_static(b"\xff\0"),
        Bytes::new(),
    ];
    let attributes = Some(FrameMap::from([(
        CmdStr::new("metadata").to_blob(),
        CmdStr::new("value").to_blob(),
    )]));
    let frame = Resp3Frame::Array {
        data: keys
            .iter()
            .cloned()
            .map(|data| Resp3Frame::BlobString {
                data,
                attributes: attributes.clone(),
            })
            .collect(),
        attributes,
    };

    assert_eq!(keys, KeysCommand::new("*").eval_response(frame).unwrap());
}

#[test]
fn test_eval_response_resp2_no_matches() {
    let keys = KeysCommand::new("missing:*").eval_response(Resp2Frame::Array(vec![])).unwrap();
    assert!(keys.is_empty());
}

#[test]
fn test_eval_response_resp3_no_matches() {
    let keys = KeysCommand::new("missing:*")
        .eval_response(Resp3Frame::Array {
            data: vec![],
            attributes: None,
        })
        .unwrap();
    assert!(keys.is_empty());
}

#[test]
fn test_eval_response_resp2_invalid_response() {
    for frame in [
        Resp2Frame::Null,
        Resp2Frame::Integer(1),
        CmdStr::new("key").to_bulk(),
        Resp2Frame::SimpleString("key".into()),
    ] {
        assert!(KeysCommand::new("*").eval_response(frame).is_err());
    }
}

#[test]
fn test_eval_response_resp3_invalid_response() {
    for frame in [
        Resp3Frame::Null,
        Resp3Frame::Number {
            data: 1,
            attributes: None,
        },
        CmdStr::new("key").to_blob(),
        CmdStr::new("key").to_simple(),
        Resp3Frame::Map {
            data: FrameMap::new(),
            attributes: None,
        },
        Resp3Frame::Set {
            data: Default::default(),
            attributes: None,
        },
    ] {
        assert!(KeysCommand::new("*").eval_response(frame).is_err());
    }
}

#[test]
fn test_eval_response_resp2_invalid_element() {
    for invalid in [
        Resp2Frame::Null,
        Resp2Frame::Integer(1),
        Resp2Frame::Array(vec![]),
        Resp2Frame::SimpleString("key".into()),
    ] {
        let frame = Resp2Frame::Array(vec![CmdStr::new("valid").to_bulk(), invalid]);
        assert!(KeysCommand::new("*").eval_response(frame).is_err());
    }
}

#[test]
fn test_eval_response_resp3_invalid_element() {
    for invalid in [
        Resp3Frame::Null,
        Resp3Frame::Number {
            data: 1,
            attributes: None,
        },
        Resp3Frame::Array {
            data: vec![],
            attributes: None,
        },
        CmdStr::new("key").to_simple(),
    ] {
        let frame = Resp3Frame::Array {
            data: vec![CmdStr::new("valid").to_blob(), invalid],
            attributes: None,
        };
        assert!(KeysCommand::new("*").eval_response(frame).is_err());
    }
}
