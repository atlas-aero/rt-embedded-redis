use crate::commands::hset::HashSetCommand;
use crate::commands::Command;
use redis_protocol::resp2::types::Frame as Resp2Frame;
use redis_protocol::resp3::types::Frame as Resp3Frame;

#[test]
fn test_encode_single_field_resp2() {
    let frame: Resp2Frame = HashSetCommand::new("my_hash".into(), "color".into(), "green".into()).encode();

    assert!(frame.is_array());
    if let Resp2Frame::Array(array) = frame {
        assert_eq!(4, array.len());
        assert_eq!("HSET", array[0].to_string().unwrap());
        assert_eq!("my_hash", array[1].to_string().unwrap());
        assert_eq!("color", array[2].to_string().unwrap());
        assert_eq!("green", array[3].to_string().unwrap());
    }
}

#[test]
fn test_encode_single_field_resp3() {
    let frame: Resp3Frame = HashSetCommand::new("my_hash".into(), "color".into(), "green".into()).encode();

    if let Resp3Frame::Array { data, attributes: _ } = frame {
        assert_eq!(4, data.len());
        assert_eq!("HSET", data[0].to_string().unwrap());
        assert_eq!("my_hash", data[1].to_string().unwrap());
        assert_eq!("color", data[2].to_string().unwrap());
        assert_eq!("green", data[3].to_string().unwrap());
    }
}

#[test]
fn test_encode_multiple_fields_resp2() {
    let frame: Resp2Frame = HashSetCommand::multiple(
        "my_hash".into(),
        [
            ("gender".into(), "male".into()),
            ("material".into(), "wood".into()),
        ],
    )
    .encode();

    if let Resp2Frame::Array(array) = frame {
        assert_eq!(6, array.len());
        assert_eq!("HSET", array[0].to_string().unwrap());
        assert_eq!("my_hash", array[1].to_string().unwrap());
        assert_eq!("gender", array[2].to_string().unwrap());
        assert_eq!("male", array[3].to_string().unwrap());
        assert_eq!("material", array[4].to_string().unwrap());
        assert_eq!("wood", array[5].to_string().unwrap());
    }
}

#[test]
fn test_encode_multiple_fields_resp3() {
    let frame: Resp3Frame = HashSetCommand::multiple(
        "my_hash".into(),
        [
            ("gender".into(), "male".into()),
            ("material".into(), "wood".into()),
        ],
    )
    .encode();

    if let Resp3Frame::Array { data, attributes: _ } = frame {
        assert_eq!(6, data.len());
        assert_eq!("HSET", data[0].to_string().unwrap());
        assert_eq!("my_hash", data[1].to_string().unwrap());
        assert_eq!("gender", data[2].to_string().unwrap());
        assert_eq!("male", data[3].to_string().unwrap());
        assert_eq!("material", data[4].to_string().unwrap());
        assert_eq!("wood", data[5].to_string().unwrap());
    }
}

#[test]
fn test_eval_response_resp2_success() {
    let command = HashSetCommand::new("my_hash".into(), "color".into(), "green".into());
    let response = command.eval_response(Resp2Frame::Integer(2));

    assert_eq!(2, response.unwrap());
}

#[test]
fn test_eval_response_resp3_success() {
    let command = HashSetCommand::new("my_hash".into(), "color".into(), "green".into());
    let response = command.eval_response(Resp3Frame::Number {
        data: 3,
        attributes: None,
    });

    assert_eq!(3, response.unwrap());
}

#[test]
fn test_eval_response_resp2_invalid_response() {
    let command = HashSetCommand::new("my_hash".into(), "color".into(), "green".into());
    let response = command.eval_response(Resp2Frame::BulkString("3".into()));

    assert!(response.is_err());
}

#[test]
fn test_eval_response_resp3_invalid_response() {
    let command = HashSetCommand::new("my_hash".into(), "color".into(), "green".into());
    let response = command.eval_response(Resp3Frame::BlobString {
        data: "test".into(),
        attributes: None,
    });

    assert!(response.is_err());
}
