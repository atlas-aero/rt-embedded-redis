use crate::commands::set::{Exclusivity, ExpirationPolicy, SetCommand};
use crate::commands::Command;
use crate::network::tests::mocks::MockFrames;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use bytes::Bytes;
use redis_protocol::resp2::types::Frame as Resp2Frame;
use redis_protocol::resp3::prelude::Frame;
use redis_protocol::resp3::types::Frame as Resp3Frame;

#[test]
fn test_encode_no_options() {
    let command = SetCommand::new("test_key", "value123");
    assert_command(vec!["SET", "test_key", "value123"], command);
}

#[test]
fn test_encode_expiration_keep() {
    let command = SetCommand::new("test_key", "value123").expires(ExpirationPolicy::Keep);
    assert_command(vec!["SET", "test_key", "value123", "KEEPTTL"], command);
}

#[test]
fn test_encode_expiration_seconds() {
    let command = SetCommand::new("test_key", "value123").expires(ExpirationPolicy::Seconds(120));
    assert_command(vec!["SET", "test_key", "value123", "EX", "120"], command);
}

#[test]
fn test_encode_expiration_milliseconds() {
    let command = SetCommand::new("test_key", "value123").expires(ExpirationPolicy::Milliseconds(1674));
    assert_command(vec!["SET", "test_key", "value123", "PX", "1674"], command);
}

#[test]
fn test_encode_expiration_timestamp_seconds() {
    let command =
        SetCommand::new("test_key", "value123").expires(ExpirationPolicy::TimestampSeconds(1648210076));
    assert_command(vec!["SET", "test_key", "value123", "EXAT", "1648210076"], command);
}

#[test]
fn test_encode_expiration_timestamp_milliseconds() {
    let command =
        SetCommand::new("test_key", "value123").expires(ExpirationPolicy::TimestampMilliseconds(4294967295));

    assert_command(vec!["SET", "test_key", "value123", "PXAT", "4294967295"], command);
}

#[test]
fn test_encode_exclusive_nx() {
    let command = SetCommand::new("test_key", "value123").set_exclusive(Exclusivity::SetIfMissing);

    assert_command(vec!["SET", "test_key", "value123", "NX"], command);
}

#[test]
fn test_encode_exclusive_xx() {
    let command = SetCommand::new("test_key", "value123").set_exclusive(Exclusivity::SetIfExists);

    assert_command(vec!["SET", "test_key", "value123", "XX"], command);
}

#[test]
fn test_encode_return_previous() {
    let command = SetCommand::new("test_key", "value123").return_previous();

    assert_command(vec!["SET", "test_key", "value123", "GET"], command);
}

#[test]
fn test_encode_expiration_exclusive() {
    let command = SetCommand::new("test_key", "value123")
        .expires(ExpirationPolicy::Seconds(140))
        .set_exclusive(Exclusivity::SetIfMissing);

    assert_command(vec!["SET", "test_key", "value123", "EX", "140", "NX"], command);
}

#[test]
fn test_encode_expiration_return_previous() {
    let command = SetCommand::new("test_key", "value123")
        .expires(ExpirationPolicy::Seconds(140))
        .return_previous();

    assert_command(vec!["SET", "test_key", "value123", "EX", "140", "GET"], command);
}

#[test]
fn test_encode_exclusive_return_previous() {
    let command = SetCommand::new("test_key", "value123")
        .set_exclusive(Exclusivity::SetIfExists)
        .return_previous();

    assert_command(vec!["SET", "test_key", "value123", "XX", "GET"], command);
}

#[test]
fn test_encode_all_options() {
    let command = SetCommand::new("test_key", "value123")
        .expires(ExpirationPolicy::Seconds(140))
        .set_exclusive(Exclusivity::SetIfMissing)
        .return_previous();

    assert_command(
        vec!["SET", "test_key", "value123", "EX", "140", "NX", "GET"],
        command,
    );
}

#[test]
fn test_eval_response_resp2_no_options_success() {
    let command = SetCommand::new("test_key", "value123");
    command.eval_response(MockFrames::ok_resp2()).unwrap();
}

#[test]
fn test_eval_response_resp3_no_options_success() {
    let command = SetCommand::new("test_key", "value123");
    command.eval_response(MockFrames::ok_resp3()).unwrap();
}

#[test]
fn test_eval_response_resp2_no_options_invalid_response() {
    let command = SetCommand::new("test_key", "value123");
    assert!(command.eval_response(Resp2Frame::Array(vec![])).is_err());
}

#[test]
fn test_eval_response_resp3_no_options_invalid_response() {
    let command = SetCommand::new("test_key", "value123");
    let frame = Resp3Frame::Array {
        data: vec![],
        attributes: None,
    };
    assert!(command.eval_response(frame).is_err());
}

#[test]
fn test_eval_response_resp2_expiration_success() {
    let command = SetCommand::new("test_key", "value123").expires(ExpirationPolicy::Seconds(120));
    command.eval_response(MockFrames::ok_resp2()).unwrap();
}

#[test]
fn test_eval_response_resp3_expiration_success() {
    let command = SetCommand::new("test_key", "value123").expires(ExpirationPolicy::Seconds(120));
    command.eval_response(MockFrames::ok_resp3()).unwrap();
}

#[test]
fn test_eval_response_resp2_expiration_invalid_response() {
    let command = SetCommand::new("test_key", "value123").expires(ExpirationPolicy::Seconds(120));
    assert!(command.eval_response(Resp2Frame::Array(vec![])).is_err());
}

#[test]
fn test_eval_response_resp3_expiration_invalid_response() {
    let command = SetCommand::new("test_key", "value123").expires(ExpirationPolicy::Seconds(120));
    let frame = Resp3Frame::Array {
        data: vec![],
        attributes: None,
    };
    assert!(command.eval_response(frame).is_err());
}

#[test]
fn test_eval_response_resp2_exclusive_success() {
    let command = SetCommand::new("test_key", "value123").set_exclusive(Exclusivity::SetIfExists);
    assert!(command.eval_response(MockFrames::ok_resp2()).unwrap().is_some());
}

#[test]
fn test_eval_response_resp3_exclusive_success() {
    let command = SetCommand::new("test_key", "value123").set_exclusive(Exclusivity::SetIfExists);
    assert!(command.eval_response(MockFrames::ok_resp3()).unwrap().is_some());
}

#[test]
fn test_eval_response_resp2_exclusive_nil() {
    let command = SetCommand::new("test_key", "value123").set_exclusive(Exclusivity::SetIfExists);
    assert!(command.eval_response(Resp2Frame::Null).unwrap().is_none());
}

#[test]
fn test_eval_response_resp3_exclusive_nil() {
    let command = SetCommand::new("test_key", "value123").set_exclusive(Exclusivity::SetIfExists);
    assert!(command.eval_response(Resp3Frame::Null).unwrap().is_none());
}

#[test]
fn test_eval_response_resp2_invalid_response() {
    let command = SetCommand::new("test_key", "value123").set_exclusive(Exclusivity::SetIfExists);
    assert!(command.eval_response(Resp2Frame::Array(vec![])).is_err());
}

#[test]
fn test_eval_response_resp3_invalid_response() {
    let command = SetCommand::new("test_key", "value123").set_exclusive(Exclusivity::SetIfExists);
    let frame = Resp3Frame::Array {
        data: vec![],
        attributes: None,
    };
    assert!(command.eval_response(frame).is_err());
}

#[test]
fn test_eval_response_resp2_get_previous_value_exists() {
    let command = SetCommand::new("test_key", "value123").return_previous();
    let response = Resp2Frame::BulkString(Bytes::from_static("Test value".as_bytes()));
    assert_eq!(
        "Test value".as_bytes(),
        command.eval_response(response).unwrap().unwrap().as_ref()
    );
}

#[test]
fn test_eval_response_resp3_get_previous_value_exists() {
    let command = SetCommand::new("test_key", "value123").return_previous();
    let response = Resp3Frame::BlobString {
        data: Bytes::from("cool_test"),
        attributes: None,
    };
    assert_eq!(
        "cool_test".as_bytes(),
        command.eval_response(response).unwrap().unwrap().as_ref()
    );
}

#[test]
fn test_eval_response_resp2_get_previous_value_missing() {
    let command = SetCommand::new("test_key", "value123").return_previous();
    assert!(command.eval_response(Resp2Frame::Null).unwrap().is_none())
}

#[test]
fn test_eval_response_resp3_get_previous_value_missing() {
    let command = SetCommand::new("test_key", "value123").return_previous();
    assert!(command.eval_response(Resp3Frame::Null).unwrap().is_none())
}

#[test]
fn test_eval_response_resp2_get_previous_invalid_response() {
    let command = SetCommand::new("test_key", "value123").return_previous();
    assert!(command.eval_response(Resp2Frame::Integer(123)).is_err())
}

#[test]
fn test_eval_response_resp3_get_previous_invalid_response() {
    let command = SetCommand::new("test_key", "value123").return_previous();
    assert!(command
        .eval_response(Resp3Frame::Boolean {
            data: false,
            attributes: None
        })
        .is_err())
}

/// Asserts RESP2 + RESP3 command arguments
fn assert_command<C>(expected: Vec<&'static str>, command: C)
where
    C: Command<Resp2Frame> + Command<Resp3Frame>,
{
    assert_resp2_command(expected.clone(), command.encode());
    assert_resp3_command(expected.clone(), command.encode());
}

fn assert_resp2_command(expected: Vec<&'static str>, frame: Resp2Frame) {
    assert!(frame.is_array());
    if let Resp2Frame::Array(array) = frame {
        assert_eq!(expected.len(), array.len());

        for item in expected.iter().enumerate() {
            assert_eq!(
                item.1.to_string(),
                array.get(item.0).unwrap().to_string().unwrap()
            );
        }
    }
}

fn assert_resp3_command(expected: Vec<&'static str>, frame: Resp3Frame) {
    assert!(frame.is_array());

    if let Frame::Array { data, attributes: _ } = frame {
        assert_eq!(expected.len(), data.len());

        for item in expected.iter().enumerate() {
            assert_eq!(item.1.to_string(), data.get(item.0).unwrap().to_string().unwrap());
        }
    }
}
