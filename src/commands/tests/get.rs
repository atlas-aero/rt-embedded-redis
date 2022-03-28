use crate::commands::get::{GetCommand, GetResponse};
use crate::commands::helpers::CmdStr;
use crate::commands::Command;
use alloc::string::ToString;
use alloc::vec;
use bytes::Bytes;
use redis_protocol::resp2::types::Frame as Resp2Frame;
use redis_protocol::resp3::types::Frame as Resp3Frame;

#[test]
fn test_encode_resp2() {
    let frame: Resp2Frame = GetCommand::new("test_key").encode();

    assert!(frame.is_array());
    match frame {
        Resp2Frame::Array(array) => {
            assert_eq!(2, array.len());
            assert_eq!("GET", array[0].to_string().unwrap());
            assert_eq!("test_key", array[1].to_string().unwrap());
        }
        _ => {}
    }
}

#[test]
fn test_encode_resp3() {
    let frame: Resp3Frame = GetCommand::new("test_key").encode();

    assert!(frame.is_array());
    match frame {
        Resp3Frame::Array { data, attributes: _ } => {
            assert_eq!(2, data.len());
            assert_eq!("GET", data[0].to_string().unwrap());
            assert_eq!("test_key", data[1].to_string().unwrap());
        }
        _ => {}
    }
}

#[test]
fn test_eval_response_resp2_key_existing() {
    let response = GetCommand::new("test_key")
        .eval_response(CmdStr::new("correct response1").to_bulk())
        .unwrap();

    assert_eq!("correct response1", response.unwrap().as_str().unwrap());
}

#[test]
fn test_eval_response_resp3_key_existing() {
    let response = GetCommand::new("test_key")
        .eval_response(CmdStr::new("correct").to_blob())
        .unwrap();

    assert_eq!("correct", response.unwrap().as_str().unwrap());
}

#[test]
fn test_eval_response_resp2_key_missing() {
    let response = GetCommand::new("test_key").eval_response(Resp2Frame::Null).unwrap();

    assert!(response.is_none());
}

#[test]
fn test_eval_response_resp3_key_missing() {
    let response = GetCommand::new("test_key").eval_response(Resp3Frame::Null).unwrap();

    assert!(response.is_none());
}

#[test]
fn test_eval_response_resp2_invalid_response() {
    let response = GetCommand::new("test_key").eval_response(Resp2Frame::Array(vec![]));

    assert!(response.is_err());
}

#[test]
fn test_eval_response_resp3_invalid_response() {
    let response = GetCommand::new("test_key").eval_response(Resp3Frame::Array {
        data: vec![],
        attributes: None,
    });

    assert!(response.is_err());
}

#[test]
fn test_response_to_bytes() {
    let inner = Bytes::from_static("test response".as_bytes());
    let response = GetResponse::new(inner);

    assert_eq!(
        Bytes::from_static("test response".as_bytes()),
        response.to_bytes()
    );
}

#[test]
fn test_response_as_string_success() {
    let inner = Bytes::from_static("test response".as_bytes());
    let response = GetResponse::new(inner);

    assert_eq!("test response".to_string(), response.as_string().unwrap());
}

#[test]
fn test_response_as_string_fail() {
    let inner = Bytes::from_static(b"\xc3\x28");
    let response = GetResponse::new(inner);

    assert!(response.as_string().is_none());
}

#[test]
fn test_response_as_str_success() {
    let inner = Bytes::from_static("test response".as_bytes());
    let response = GetResponse::new(inner);

    assert_eq!("test response", response.as_str().unwrap());
}

#[test]
fn test_response_as_str_fail() {
    let inner = Bytes::from_static(b"\xc3\x28");
    let response = GetResponse::new(inner);

    assert!(response.as_str().is_none());
}
