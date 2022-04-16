use crate::commands::helpers::CmdStr;
use crate::commands::ping::PingCommand;
use crate::commands::Command;
use redis_protocol::resp2::types::Frame as Resp2Frame;
use redis_protocol::resp3::types::Frame as Resp3Frame;

#[test]
fn test_encode_default_resp2() {
    let frame: Resp2Frame = PingCommand::new(None).encode();

    assert!(frame.is_array());
    if let Resp2Frame::Array(array) = frame {
        assert_eq!(1, array.len());
        assert_eq!("PING", array[0].to_string().unwrap());
    }
}

#[test]
fn test_encode_default_resp3() {
    let frame: Resp3Frame = PingCommand::new(None).encode();

    assert!(frame.is_array());
    if let Resp3Frame::Array { data, attributes: _ } = frame {
        assert_eq!(1, data.len());
        assert_eq!("PING", data[0].to_string().unwrap());
    }
}

#[test]
fn test_encode_with_arg_resp2() {
    let frame: Resp2Frame = PingCommand::new(Some("hello world".into())).encode();

    assert!(frame.is_array());
    if let Resp2Frame::Array(array) = frame {
        assert_eq!(2, array.len());
        assert_eq!("PING", array[0].to_string().unwrap());
        assert_eq!("hello world", array[1].to_string().unwrap());
    }
}

#[test]
fn test_encode_with_arg_resp3() {
    let frame: Resp3Frame = PingCommand::new(Some("hello world".into())).encode();

    assert!(frame.is_array());
    if let Resp3Frame::Array { data, attributes: _ } = frame {
        assert_eq!(2, data.len());
        assert_eq!("PING", data[0].to_string().unwrap());
        assert_eq!("hello world", data[1].to_string().unwrap());
    }
}

#[test]
fn test_eval_response_default_pong() {
    PingCommand::new(None).eval_response(CmdStr::new("PONG").to_blob()).unwrap();
}

#[test]
fn test_eval_response_default_error() {
    let response = PingCommand::new(None).eval_response(CmdStr::new("OTHER").to_blob());
    assert!(response.is_err());
}

#[test]
fn test_eval_response_argument_ok() {
    PingCommand::new(Some("hello world".into()))
        .eval_response(CmdStr::new("hello world").to_blob())
        .unwrap();
}

#[test]
fn test_eval_response_argument_error() {
    let response = PingCommand::new(Some("hello world".into())).eval_response(CmdStr::new("PONG").to_blob());
    assert!(response.is_err());
}
