use crate::commands::hello::HelloCommand;
use crate::commands::helpers::CmdStr;
use crate::commands::Command;
use crate::network::tests::mocks::MockFrames;
use alloc::vec;
use redis_protocol::resp3::prelude::Frame;
use redis_protocol::resp3::types::RespVersion;

#[test]
fn test_encode() {
    let command = HelloCommand {};
    let frame = command.encode();

    match frame {
        Frame::Hello { version, auth } => {
            assert_eq!(RespVersion::RESP3, version);
            assert_eq!(None, auth);
        }
        _ => {
            panic!("Unexpected frame type")
        }
    }
}

#[test]
fn test_eval_response_correct() {
    let command = HelloCommand {};
    let frame = MockFrames::hello();

    let result = command.eval_response(frame).unwrap();
    assert_eq!("redis", result.server);
    assert_eq!("6.0.0", result.version);
    assert_eq!(3, result.protocol);
    assert_eq!(10, result.id);
    assert_eq!("standalone", result.mode);
    assert_eq!("master", result.role);
    assert!(result.modules.is_empty());
}

#[test]
fn test_eval_response_server_missing() {
    assert_missing_key("server");
}

#[test]
fn test_eval_response_server_not_string() {
    assert_not_string("server");
}

#[test]
fn test_eval_response_version_missing() {
    assert_missing_key("version");
}

#[test]
fn test_eval_response_version_not_string() {
    assert_not_string("version");
}

#[test]
fn test_eval_response_proto_missing() {
    assert_missing_key("proto");
}

#[test]
fn test_eval_response_proto_not_integer() {
    assert_not_integer("proto");
}

#[test]
fn test_eval_response_id_missing() {
    assert_missing_key("id");
}

#[test]
fn test_eval_response_id_not_integer() {
    assert_not_integer("id");
}

#[test]
fn test_eval_response_mode_missing() {
    assert_missing_key("mode");
}

#[test]
fn test_eval_response_mode_not_string() {
    assert_not_string("mode");
}

#[test]
fn test_eval_response_role_missing() {
    assert_missing_key("role");
}

#[test]
fn test_eval_response_role_not_string() {
    assert_not_string("server");
}

#[test]
fn test_eval_response_modules_missing() {
    assert_missing_key("modules");
}

#[test]
fn test_eval_response_modules_not_array() {
    assert_not_array("modules");
}

/// Removes the given key from the frame
fn remove_key(frame: Frame, key: &str) -> Frame {
    match frame {
        Frame::Map {
            mut data,
            attributes: _,
        } => {
            data.remove(&CmdStr::new(key).to_blob());
            Frame::Map {
                data,
                attributes: None,
            }
        }
        frame => frame,
    }
}

/// Replaces the associates element by an empty array
fn add_empty_array(frame: Frame, key: &str) -> Frame {
    match frame {
        Frame::Map {
            mut data,
            attributes: _,
        } => {
            data.remove(&CmdStr::new(key).to_blob());
            data.insert(
                CmdStr::new(key).to_blob(),
                Frame::Array {
                    data: vec![],
                    attributes: None,
                },
            );
            Frame::Map {
                data,
                attributes: None,
            }
        }
        frame => frame,
    }
}

/// Replaces the associates element by a dummy string
fn add_dummy_string(frame: Frame, key: &str) -> Frame {
    match frame {
        Frame::Map {
            mut data,
            attributes: _,
        } => {
            data.remove(&CmdStr::new(key).to_blob());
            data.insert(CmdStr::new(key).to_blob(), CmdStr::new("dummy").to_blob());
            Frame::Map {
                data,
                attributes: None,
            }
        }
        frame => frame,
    }
}

fn assert_missing_key(key: &str) {
    let command = HelloCommand {};
    let frame = remove_key(MockFrames::hello(), key);

    assert!(command.eval_response(frame).is_err())
}

fn assert_not_string(key: &str) {
    let command = HelloCommand {};
    let frame = add_empty_array(MockFrames::hello(), key);

    assert!(command.eval_response(frame).is_err())
}

fn assert_not_integer(key: &str) {
    let command = HelloCommand {};
    let frame = add_empty_array(MockFrames::hello(), key);

    assert!(command.eval_response(frame).is_err())
}

fn assert_not_array(key: &str) {
    let command = HelloCommand {};
    let frame = add_dummy_string(MockFrames::hello(), key);

    assert!(command.eval_response(frame).is_err())
}
