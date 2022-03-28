//! Helpers for command abstraction.
use alloc::borrow::ToOwned;
use alloc::string::{String, ToString};
use bytes::Bytes;
use redis_protocol::resp2::types::Frame as Resp2Frame;
use redis_protocol::resp3::types::{Frame as Resp3Frame, FrameMap};

/// Helper for casting Strings to frame types
pub struct CmdStr<'a> {
    inner: &'a str,
}

impl<'a> CmdStr<'a> {
    pub fn new(inner: &'a str) -> Self {
        CmdStr { inner }
    }

    pub fn to_blob(self) -> Resp3Frame {
        Resp3Frame::BlobString {
            data: Bytes::from(self.inner.to_owned()),
            attributes: None,
        }
    }

    pub fn to_simple(self) -> Resp3Frame {
        Resp3Frame::SimpleString {
            data: Bytes::from(self.inner.to_owned()),
            attributes: None,
        }
    }
}

impl CmdStr<'static> {
    pub fn to_bulk(self) -> Resp2Frame {
        Resp2Frame::BulkString(Bytes::from(self.inner))
    }
}

/// Helper for casting response integers to frame types
pub struct RespInt {
    inner: i64,
}

impl RespInt {
    pub fn new(inner: i64) -> Self {
        RespInt { inner }
    }

    pub fn to_number(&self) -> Resp3Frame {
        Resp3Frame::Number {
            data: self.inner,
            attributes: None,
        }
    }
}

/// Helper for casting integers to frame types
pub struct CmdInt {
    inner: usize,
}

impl CmdInt {
    pub fn new(inner: usize) -> Self {
        CmdInt { inner }
    }

    pub fn to_blob(&self) -> Resp3Frame {
        Resp3Frame::BlobString {
            data: Bytes::from(self.inner.to_string()),
            attributes: None,
        }
    }

    pub fn to_bulk_string(&self) -> Resp2Frame {
        Resp2Frame::BulkString(Bytes::from(self.inner.to_string()))
    }
}

/// Helper for casting Bytes to frames
pub struct CmdBytes {
    inner: Bytes,
}

impl CmdBytes {
    pub fn new(bytes: &Bytes) -> Self {
        CmdBytes { inner: bytes.clone() }
    }

    pub fn to_blob(&self) -> Resp3Frame {
        Resp3Frame::BlobString {
            data: self.inner.clone(),
            attributes: None,
        }
    }
}

/// Helper for finding & casting map elements
pub struct RespMap<'a> {
    inner: &'a FrameMap,
}

impl<'a> RespMap<'a> {
    pub fn new(inner: &'a FrameMap) -> Self {
        RespMap { inner }
    }

    pub fn find_string(&self, key: &str) -> Option<String> {
        self.inner.get(&CmdStr::new(key).to_blob())?.to_string()
    }

    pub fn find_integer(&self, key: &str) -> Option<i64> {
        let element = self.inner.get(&CmdStr::new(key).to_blob())?;

        match element {
            Resp3Frame::Number { data, attributes: _ } => Some(*data),
            _ => None,
        }
    }
}
