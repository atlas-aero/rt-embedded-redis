use crate::commands::publish::PublishCommand;
use crate::commands::Command;
use redis_protocol::resp2::types::Frame as Resp2Frame;
use redis_protocol::resp3::types::Frame as Resp3Frame;

#[test]
fn test_encode_resp2() {
    let command = PublishCommand::new("test_channel", "test_message");
    let frame: Resp2Frame = command.encode();

    assert!(frame.is_array());
    if let Resp2Frame::Array(array) = frame {
        assert_eq!(3, array.len());
        assert_eq!("PUBLISH", array[0].to_string().unwrap());
        assert_eq!("test_channel", array[1].to_string().unwrap());
        assert_eq!("test_message", array[2].to_string().unwrap());
    }
}

#[test]
fn test_encode_resp3() {
    let command = PublishCommand::new("test_channel", "test_message");
    let frame: Resp3Frame = command.encode();

    assert!(frame.is_array());
    if let Resp3Frame::Array { data, attributes: _ } = frame {
        assert_eq!(3, data.len());
        assert_eq!("PUBLISH", data[0].to_string().unwrap());
        assert_eq!("test_channel", data[1].to_string().unwrap());
        assert_eq!("test_message", data[2].to_string().unwrap());
    }
}

#[test]
fn test_eval_response_resp2_success() {
    let command = PublishCommand::new("test_channel", "test_message");
    let response = command.eval_response(Resp2Frame::Integer(14));

    assert_eq!(14, response.unwrap());
}

#[test]
fn test_eval_response_resp3_success() {
    let command = PublishCommand::new("test_channel", "test_message");
    let response = command.eval_response(Resp3Frame::Number {
        data: 3,
        attributes: None,
    });

    assert_eq!(3, response.unwrap());
}

#[test]
fn test_eval_response_resp2_invalid_response() {
    let command = PublishCommand::new("test_channel", "test_message");
    let response = command.eval_response(Resp2Frame::BulkString("3".into()));

    assert!(response.is_err());
}

#[test]
fn test_eval_response_resp3_invalid_response() {
    let command = PublishCommand::new("test_channel", "test_message");
    let response = command.eval_response(Resp3Frame::BlobString {
        data: "test".into(),
        attributes: None,
    });

    assert!(response.is_err());
}
