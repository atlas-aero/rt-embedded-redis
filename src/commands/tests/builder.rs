use crate::commands::builder::ToBytesMap;
use bytes::Bytes;
use redis_protocol::resp2::types::BytesFrame as Resp2Frame;
use redis_protocol::resp3::types::{BytesFrame as Resp3Frame, FrameMap};

#[test]
fn to_bytes_map_resp2_simple_string() {
    let frame: Resp2Frame = Resp2Frame::Array(vec![
        Resp2Frame::SimpleString("color".into()),
        Resp2Frame::SimpleString("green".into()),
        Resp2Frame::SimpleString("material".into()),
        Resp2Frame::SimpleString("wood".into()),
    ]);
    let map = frame.to_map().unwrap();

    assert_eq!(2, map.len());
    assert_eq!("green", map.get(&Bytes::from_static(b"color")).unwrap());
    assert_eq!("wood", map.get(&Bytes::from_static(b"material")).unwrap());
}

#[test]
fn to_bytes_map_resp2_bulk_string() {
    let frame: Resp2Frame = Resp2Frame::Array(vec![
        Resp2Frame::BulkString("color".into()),
        Resp2Frame::BulkString("green".into()),
        Resp2Frame::BulkString("material".into()),
        Resp2Frame::BulkString("wood".into()),
    ]);
    let map = frame.to_map().unwrap();

    assert_eq!(2, map.len());
    assert_eq!("green", map.get(&Bytes::from_static(b"color")).unwrap());
    assert_eq!("wood", map.get(&Bytes::from_static(b"material")).unwrap());
}

#[test]
fn to_bytes_map_resp2_no_array() {
    assert!(Resp2Frame::SimpleString("test".into()).to_map().is_none());
}

#[test]
fn to_bytes_map_resp2_missing_value() {
    let frame: Resp2Frame = Resp2Frame::Array(vec![
        Resp2Frame::SimpleString("color".into()),
        Resp2Frame::SimpleString("green".into()),
        Resp2Frame::SimpleString("material".into()),
    ]);

    assert!(frame.to_map().is_none());
}

#[test]
fn to_bytes_map_resp2_field_not_string() {
    let frame: Resp2Frame = Resp2Frame::Array(vec![
        Resp2Frame::Array(vec![]),
        Resp2Frame::SimpleString("green".into()),
    ]);

    assert!(frame.to_map().is_none());
}

#[test]
fn to_bytes_map_resp2_value_not_string() {
    let frame: Resp2Frame = Resp2Frame::Array(vec![
        Resp2Frame::SimpleString("color".into()),
        Resp2Frame::Array(vec![]),
    ]);

    assert!(frame.to_map().is_none());
}

#[test]
fn to_bytes_map_resp3_simple_string() {
    let frame: Resp3Frame = Resp3Frame::Map {
        data: FrameMap::from([
            (
                Resp3Frame::SimpleString {
                    data: "color".into(),
                    attributes: None,
                },
                Resp3Frame::SimpleString {
                    data: "green".into(),
                    attributes: None,
                },
            ),
            (
                Resp3Frame::SimpleString {
                    data: "material".into(),
                    attributes: None,
                },
                Resp3Frame::SimpleString {
                    data: "wood".into(),
                    attributes: None,
                },
            ),
        ]),
        attributes: None,
    };
    let map = frame.to_map().unwrap();

    assert_eq!(2, map.len());
    assert_eq!("green", map.get(&Bytes::from_static(b"color")).unwrap());
    assert_eq!("wood", map.get(&Bytes::from_static(b"material")).unwrap());
}

#[test]
fn to_bytes_map_resp3_blob_string() {
    let frame: Resp3Frame = Resp3Frame::Map {
        data: FrameMap::from([
            (
                Resp3Frame::BlobString {
                    data: "color".into(),
                    attributes: None,
                },
                Resp3Frame::BlobString {
                    data: "green".into(),
                    attributes: None,
                },
            ),
            (
                Resp3Frame::BlobString {
                    data: "material".into(),
                    attributes: None,
                },
                Resp3Frame::BlobString {
                    data: "wood".into(),
                    attributes: None,
                },
            ),
        ]),
        attributes: None,
    };
    let map = frame.to_map().unwrap();

    assert_eq!(2, map.len());
    assert_eq!("green", map.get(&Bytes::from_static(b"color")).unwrap());
    assert_eq!("wood", map.get(&Bytes::from_static(b"material")).unwrap());
}

#[test]
fn to_bytes_map_resp3_no_array() {
    assert!(Resp3Frame::BlobString {
        data: "test".into(),
        attributes: None,
    }
    .to_map()
    .is_none());
}

#[test]
fn to_bytes_map_resp3_field_not_string() {
    let frame: Resp3Frame = Resp3Frame::Map {
        data: FrameMap::from([(
            Resp3Frame::Number {
                data: 0,
                attributes: None,
            },
            Resp3Frame::SimpleString {
                data: "green".into(),
                attributes: None,
            },
        )]),
        attributes: None,
    };
    assert!(frame.to_map().is_none());
}

#[test]
fn to_bytes_map_resp3_value_not_string() {
    let frame: Resp3Frame = Resp3Frame::Map {
        data: FrameMap::from([(
            Resp3Frame::SimpleString {
                data: "color".into(),
                attributes: None,
            },
            Resp3Frame::Number {
                data: 0,
                attributes: None,
            },
        )]),
        attributes: None,
    };
    assert!(frame.to_map().is_none());
}
