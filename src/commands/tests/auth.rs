use crate::commands::auth::AuthCommand;
use crate::commands::Command;
use alloc::vec;
use bytes::Bytes;
use redis_protocol::resp2::prelude::Frame as Resp2Frame;
use redis_protocol::resp3::prelude::{Frame as Resp3Frame, Frame};

#[test]
fn test_resp2_encode_no_username() {
    let command = AuthCommand::new(None as Option<Bytes>, "secret123!");
    let frame: Resp2Frame = command.encode();

    assert!(frame.is_array());
    if let Resp2Frame::Array(array) = frame {
        assert_eq!(2, array.len());
        assert_eq!("AUTH", array.get(0).unwrap().to_string().unwrap());
        assert_eq!("secret123!", array.get(1).unwrap().to_string().unwrap());
    }
}

#[test]
fn test_resp2_encode_username() {
    let command = AuthCommand::new(Some("test_user"), "secret123!");
    let frame: Resp2Frame = command.encode();

    assert!(frame.is_array());
    if let Resp2Frame::Array(array) = frame {
        assert_eq!(3, array.len());
        assert_eq!("AUTH", array.get(0).unwrap().to_string().unwrap());
        assert_eq!("test_user", array.get(1).unwrap().to_string().unwrap());
        assert_eq!("secret123!", array.get(2).unwrap().to_string().unwrap());
    }
}

#[test]
fn test_resp3_encode_no_username() {
    let command = AuthCommand::new(None as Option<Bytes>, "secret123!");
    let frame: Resp3Frame = command.encode();

    assert!(frame.is_array());
    if let Frame::Array { data, attributes } = frame {
        assert_eq!(2, data.len());
        assert!(attributes.is_none());

        assert_eq!("AUTH", data.get(0).unwrap().to_string().unwrap());
        assert_eq!("secret123!", data.get(1).unwrap().to_string().unwrap());
    }
}

#[test]
fn test_resp3_encode_username() {
    let command = AuthCommand::new(Some("user01"), "secret123!");
    let frame: Resp3Frame = command.encode();

    assert!(frame.is_array());
    if let Frame::Array { data, attributes } = frame {
        assert_eq!(3, data.len());
        assert!(attributes.is_none());

        assert_eq!("AUTH", data.get(0).unwrap().to_string().unwrap());
        assert_eq!("user01", data.get(1).unwrap().to_string().unwrap());
        assert_eq!("secret123!", data.get(2).unwrap().to_string().unwrap());
    }
}

#[test]
fn test_resp2_eval_response_string_ok() {
    let command = AuthCommand::new(None as Option<Bytes>, "secret123!");
    let frame = Resp2Frame::SimpleString(Bytes::from("OK"));

    assert!(command.eval_response(frame).is_ok());
}

#[test]
fn test_resp3_eval_response_string_ok() {
    let command = AuthCommand::new(None as Option<Bytes>, "secret123!");
    let frame = Resp3Frame::SimpleString {
        data: Bytes::from("OK"),
        attributes: None,
    };

    assert!(command.eval_response(frame).is_ok());
}

#[test]
fn test_resp2_eval_response_string_not_ok() {
    let command = AuthCommand::new(None as Option<Bytes>, "secret123!");
    let frame = Resp2Frame::SimpleString(Bytes::from("Other"));

    assert!(command.eval_response(frame).is_err());
}

#[test]
fn test_resp3_eval_response_string_not_ok() {
    let command = AuthCommand::new(None as Option<Bytes>, "secret123!");
    let frame = Resp3Frame::SimpleString {
        data: Bytes::from("Other"),
        attributes: None,
    };

    assert!(command.eval_response(frame).is_err());
}

#[test]
fn test_resp2_eval_response_unexpected_response_type() {
    let command = AuthCommand::new(None as Option<Bytes>, "secret123!");
    let frame = Resp2Frame::Array(vec![]);

    assert!(command.eval_response(frame).is_err());
}

#[test]
fn test_resp3_eval_response_unexpected_response_type() {
    let command = AuthCommand::new(None as Option<Bytes> as Option<Bytes>, "secret123!");
    let frame = Resp3Frame::Array {
        data: vec![],
        attributes: None,
    };

    assert!(command.eval_response(frame).is_err());
}
