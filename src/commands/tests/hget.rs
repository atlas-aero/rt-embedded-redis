use crate::commands::helpers::CmdStr;
use crate::commands::hget::HashGetCommand;
use crate::commands::Command;
use redis_protocol::resp2::types::Frame as Resp2Frame;
use redis_protocol::resp3::types::Frame as Resp3Frame;

#[test]
fn test_encode_resp2() {
    let frame: Resp2Frame = HashGetCommand::new("my_hash", "color").encode();

    assert!(frame.is_array());
    if let Resp2Frame::Array(array) = frame {
        assert_eq!(3, array.len());
        assert_eq!("HGET", array[0].to_string().unwrap());
        assert_eq!("my_hash", array[1].to_string().unwrap());
        assert_eq!("color", array[2].to_string().unwrap());
    }
}

#[test]
fn test_encode_resp3() {
    let frame: Resp3Frame = HashGetCommand::new("my_hash", "color").encode();

    assert!(frame.is_array());
    if let Resp3Frame::Array { data, attributes: _ } = frame {
        assert_eq!(3, data.len());
        assert_eq!("HGET", data[0].to_string().unwrap());
        assert_eq!("my_hash", data[1].to_string().unwrap());
        assert_eq!("color", data[2].to_string().unwrap());
    }
}

#[test]
fn test_eval_response_resp2_key_existing() {
    let response = HashGetCommand::new("my_hash", "color")
        .eval_response(CmdStr::new("correct response1").to_bulk())
        .unwrap();

    assert_eq!("correct response1", response.unwrap().as_str().unwrap());
}

#[test]
fn test_eval_response_resp3_key_existing() {
    let response = HashGetCommand::new("my_hash", "color")
        .eval_response(CmdStr::new("correct").to_blob())
        .unwrap();

    assert_eq!("correct", response.unwrap().as_str().unwrap());
}

#[test]
fn test_eval_response_resp2_key_missing() {
    let response = HashGetCommand::new("my_hash", "color").eval_response(Resp2Frame::Null).unwrap();

    assert!(response.is_none());
}

#[test]
fn test_eval_response_resp3_key_missing() {
    let response = HashGetCommand::new("my_hash", "color").eval_response(Resp3Frame::Null).unwrap();

    assert!(response.is_none());
}

#[test]
fn test_eval_response_resp2_invalid_response() {
    let response = HashGetCommand::new("my_hash", "color").eval_response(Resp2Frame::Array(vec![]));

    assert!(response.is_err());
}

#[test]
fn test_eval_response_resp3_invalid_response() {
    let response = HashGetCommand::new("my_hash", "color").eval_response(Resp3Frame::Array {
        data: vec![],
        attributes: None,
    });

    assert!(response.is_err());
}
