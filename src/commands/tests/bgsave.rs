use crate::commands::bgsave::BackgroundSaveCommand;
use crate::commands::Command;
use bytes::Bytes;
use redis_protocol::resp2::types::Frame as Resp2Frame;
use redis_protocol::resp3::types::Frame as Resp3Frame;

#[test]
fn test_encode_resp2_default() {
    let frame: Resp2Frame = BackgroundSaveCommand::default().encode();

    assert!(frame.is_array());
    if let Resp2Frame::Array(array) = frame {
        assert_eq!(1, array.len());
        assert_eq!("BGSAVE", array[0].to_string().unwrap());
    }
}

#[test]
fn test_encode_resp2_schedule() {
    let frame: Resp2Frame = BackgroundSaveCommand::new(true).encode();

    assert!(frame.is_array());
    if let Resp2Frame::Array(array) = frame {
        assert_eq!(2, array.len());
        assert_eq!("BGSAVE", array[0].to_string().unwrap());
        assert_eq!("SCHEDULE", array[1].to_string().unwrap());
    }
}

#[test]
fn test_encode_resp3_default() {
    let frame: Resp3Frame = BackgroundSaveCommand::default().encode();

    assert!(frame.is_array());
    if let Resp3Frame::Array { data, attributes: _ } = frame {
        assert_eq!(1, data.len());
        assert_eq!("BGSAVE", data[0].to_string().unwrap());
    }
}

#[test]
fn test_encode_resp3_schedule() {
    let frame: Resp3Frame = BackgroundSaveCommand::new(true).encode();

    assert!(frame.is_array());
    if let Resp3Frame::Array { data, attributes: _ } = frame {
        assert_eq!(2, data.len());
        assert_eq!("BGSAVE", data[0].to_string().unwrap());
        assert_eq!("SCHEDULE", data[1].to_string().unwrap());
    }
}

#[test]
fn test_eval_response_resp2() {
    BackgroundSaveCommand::default()
        .eval_response(Resp2Frame::SimpleString(Bytes::from_static(
            b"Background saving started",
        )))
        .unwrap();
}

#[test]
fn test_eval_response_resp3() {
    BackgroundSaveCommand::default()
        .eval_response(Resp3Frame::SimpleString {
            data: Bytes::from_static(b"Background saving started"),
            attributes: None,
        })
        .unwrap();
}
