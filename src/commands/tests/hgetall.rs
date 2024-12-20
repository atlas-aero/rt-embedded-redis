use crate::commands::hgetall::HashGetAllCommand;
use crate::commands::Command;
use redis_protocol::resp2::types::{BytesFrame as Resp2Frame, Resp2Frame as _};
use redis_protocol::resp3::types::FrameMap;
use redis_protocol::resp3::types::{BytesFrame as Resp3Frame, Resp3Frame as _};

#[test]
fn test_encode_resp2() {
    let frame: Resp2Frame = HashGetAllCommand::new("my_hash").encode();

    assert!(matches!(frame, Resp2Frame::Array(_)));
    if let Resp2Frame::Array(array) = frame {
        assert_eq!(2, array.len());
        assert_eq!("HGETALL", array[0].to_string().unwrap());
        assert_eq!("my_hash", array[1].to_string().unwrap());
    }
}

#[test]
fn test_encode_resp3() {
    let frame: Resp3Frame = HashGetAllCommand::new("my_hash").encode();

    matches!(frame, Resp3Frame::Array { .. });
    if let Resp3Frame::Array { data, attributes: _ } = frame {
        assert_eq!(2, data.len());
        assert_eq!("HGETALL", data[0].to_string().unwrap());
        assert_eq!("my_hash", data[1].to_string().unwrap());
    }
}

#[test]
fn test_eval_response_resp2_key_existing() {
    let response = HashGetAllCommand::new("my_hash")
        .eval_response(Resp2Frame::Array(vec![
            Resp2Frame::SimpleString("color".into()),
            Resp2Frame::SimpleString("green".into()),
        ]))
        .unwrap();

    assert_eq!("green", response.unwrap().get_str("color").unwrap());
}

#[test]
fn test_eval_response_resp3_key_existing() {
    let response = HashGetAllCommand::new("my_hash")
        .eval_response(Resp3Frame::Map {
            data: FrameMap::from([(
                Resp3Frame::BlobString {
                    data: "color".into(),
                    attributes: None,
                },
                Resp3Frame::BlobString {
                    data: "green".into(),
                    attributes: None,
                },
            )]),
            attributes: None,
        })
        .unwrap();

    assert_eq!("green", response.unwrap().get_str("color").unwrap());
}

#[test]
fn test_eval_response_resp2_key_missing() {
    let response = HashGetAllCommand::new("my_hash")
        .eval_response(Resp2Frame::Array(vec![]))
        .unwrap();

    assert!(response.is_none());
}

#[test]
fn test_eval_response_resp3_key_missing() {
    let response = HashGetAllCommand::new("my_hash")
        .eval_response(Resp3Frame::Map {
            data: Default::default(),
            attributes: None,
        })
        .unwrap();

    assert!(response.is_none());
}

#[test]
fn test_eval_response_resp2_invalid_response() {
    let response = HashGetAllCommand::new("my_hash").eval_response(Resp2Frame::SimpleString("wrong".into()));

    assert!(response.is_err());
}

#[test]
fn test_eval_response_resp3_invalid_response() {
    let response = HashGetAllCommand::new("my_hash").eval_response(Resp3Frame::SimpleString {
        data: "wrong".into(),
        attributes: None,
    });

    assert!(response.is_err());
}
