use crate::commands::builder::CommandBuilder;
use crate::commands::Command;
use bytes::Bytes;
use redis_protocol::resp2::types::{BytesFrame as Resp2Frame, Resp2Frame as _};
use redis_protocol::resp3::types::{BytesFrame as Resp3Frame, Resp3Frame as _};

#[test]
fn test_encode_resp2() {
    let command = CommandBuilder::new("ECHO").arg_static("Hello World!").to_command();
    let frame: Resp2Frame = command.encode();

    assert!(matches!(frame, Resp2Frame::Array(_)));
    if let Resp2Frame::Array(array) = frame {
        assert_eq!(2, array.len());
        assert_eq!("ECHO", array[0].to_string().unwrap());
        assert_eq!("Hello World!", array[1].to_string().unwrap());
    }
}

#[test]
fn test_encode_resp3() {
    let command = CommandBuilder::new("ECHO").arg_static("Hello World!").to_command();
    let frame: Resp3Frame = command.encode();

    matches!(frame, Resp3Frame::Array { .. });
    if let Resp3Frame::Array { data, attributes: _ } = frame {
        assert_eq!(2, data.len());
        assert_eq!("ECHO", data[0].to_string().unwrap());
        assert_eq!("Hello World!", data[1].to_string().unwrap());
    }
}

#[test]
fn test_eval_response_resp2() {
    let command = CommandBuilder::new("ECHO").arg_static("Hello World!").to_command();
    let frame = Resp2Frame::BulkString(Bytes::from_static("correct_response".as_bytes()));

    let result = command.eval_response(frame).unwrap();
    assert!(result.as_str().is_some());
    assert_eq!("correct_response", result.to_string().unwrap());
}

#[test]
fn test_eval_response_resp3() {
    let command = CommandBuilder::new("ECHO").arg_static("Hello World!").to_command();
    let frame = Resp3Frame::BlobString {
        data: Bytes::from_static("correct_response".as_bytes()),
        attributes: None,
    };

    let result = command.eval_response(frame).unwrap();
    assert!(matches!(result, Resp3Frame::BlobString { .. }));
    assert_eq!("correct_response", result.to_string().unwrap());
}
