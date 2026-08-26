use crate::commands::keys::KeysCommand;
use crate::commands::Command;
use bytes::Bytes;
use redis_protocol::resp2::types::BytesFrame as Resp2Frame;
use redis_protocol::resp2::types::BytesFrame as Resp3Frame;

#[test]
fn test_encode_resp2(){
    let frame: Resp2Frame = KeysCommand::new("keys.*").encode();

    match frame {
        Resp2Frame::Array(array) => {
            assert_eq!(array.len(), 2);
            assert_eq!(array[0].to_string().unwrap(), "KEYS");
            assert_eq!(array[1].to_string().unwrap(), "keys.*");
        }
        _ => panic!("Expected RESP2 array frame"),
    }
}

#[test]
fn test_encode_resp3(){
    let frame: Resp3Frame = KeysCommand::new("keys.*").encode();

    match frame {
        // data contains the actual array elements.
        // attributes contains optional RESP3 metadata.
        // attributes: _ means: “This field exists, but ignore its value.”
        // The .. means to ignore all remaining fields
        Resp3Frame::Array { data, .. } => {
            assert_eq!(array.len(), 2);
            assert_eq!(array[0].to_string().unwrap(), "KEYS");
            assert_eq!(array[1].to_string().unwrap(), "keys.*");
        }
        _ => panic!("Expected RESP3 array frame"),
    }
}

#[test]
fn test_eval_response_resp2_keys_found(){
    let response = KeysCommand::new("keys:*")
    .eval_response(Resp2Frame::Array(vec![
        Resp2Frame::BulkString(Bytes::from_static(b"keys:1")),
        Resp2Frame::BulkString(Bytes::from_static(b"keys:2"))
    ]))
    .unwrap();

    assert_eq!(response.len(), 2);
    assert_eq!(
        response.as_slice()[0],
        Bytes::from_static(b"keys:1")
    );
    assert_eq!(
        response.as_slice()[1],
        Bytes::from_static(b"keys:2")
    );
}

#[test]
fn test_eval_response_resp3_keys_found(){
    let response = KeysCommand::new("keys:*")
        .eval_response(Resp3Frame::Array{
            data: vec![
                Resp3Frame::BlobString {
                    data: Bytes::from_static(b"keys:1"),
                    attributes: None,
                },
                Resp3Frame::BlobString {
                    data: Bytes::from_static(b"keys:2"),
                    attributes: None,
                },
            ],
            attributes: None,
        })
        .unwrap();

    assert_eq!(response.len(), 2);
    assert_eq!(
        response.as_slice()[0],
        Bytes::from_static(b"keys:1")
    );
    assert_eq!(
        response.as_slice()[1],
        Bytes::from_static(b"keys:2")
    );
}
