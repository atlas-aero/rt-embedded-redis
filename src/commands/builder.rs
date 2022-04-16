//! Builder for constructing RESP2/3 frames
//!
//! Generic wrapper mainly used as helper for creating RESP frames.
//! However, it can also be used to execute custom/arbitrary commands. See [CustomCommand](crate::commands::custom)  for more details.
//!
//! # Creating generic frames
//! The following example demonstrates the creation of command frame for [HGET](https://redis.io/commands/hget/).
//! ```
//! use embedded_redis::commands::builder::CommandBuilder;
//! use redis_protocol::resp2::types::Frame as Resp2Frame;
//!
//! let _frame: Resp2Frame = CommandBuilder::new("HGET")
//!     .arg_static("field1")
//!     .arg_static("foo")
//!     .into();
//! ```
//! # Improved performance
//! For best performance, especially with large amounts of data, it is recommended to use [Bytes](<https://docs.rs/bytes/latest/bytes/>).
//! ```
//!# use bytes::Bytes;
//! use embedded_redis::commands::builder::CommandBuilder;
//!# use redis_protocol::resp2::types::Frame as Resp2Frame;
//!#
//! // Using Bytes avoids data copy, as clone() is shallow
//! let value = Bytes::from_static("Large value".as_bytes());
//!
//! let _frame: Resp2Frame = CommandBuilder::new("HSET")
//!     .arg_static("myhash")
//!     .arg_static("field1")
//!     .arg(&value)
//!     .into();
//! ```
use crate::commands::custom::CustomCommand;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use bytes::Bytes;
use redis_protocol::resp2::types::Frame as Resp2Frame;
use redis_protocol::resp3::types::Frame as Resp3Frame;

/// Builder for constructing RESP2/3 frames
#[derive(Clone, Default)]
pub struct CommandBuilder {
    pub(crate) elements: Vec<Bytes>,
}

impl CommandBuilder {
    pub fn new(keyword: &'static str) -> Self {
        CommandBuilder {
            elements: vec![Bytes::from_static(keyword.as_bytes())],
        }
    }

    /// Converts builder to command ready for being sent by Client
    pub fn to_command(self) -> CustomCommand {
        self.into()
    }

    /// Adds a static argument
    pub fn arg_static(mut self, arg: &'static str) -> Self {
        self.elements.push(Bytes::from_static(arg.as_bytes()));
        self
    }

    /// Adds a static argument
    pub fn arg_static_option(mut self, arg: Option<&'static str>) -> Self {
        if let Some(arg_str) = arg {
            self.elements.push(Bytes::from_static(arg_str.as_bytes()));
        }
        self
    }

    /// Adds cased string of uint
    pub fn arg_uint(mut self, arg: usize) -> Self {
        self.elements.push(Bytes::from(arg.to_string()));
        self
    }

    /// Adds a byte argument
    /// Note: Besides static, the most efficient way caused by the nature how Bytes cloning is working
    pub fn arg(mut self, arg: &Bytes) -> Self {
        self.elements.push(arg.clone());
        self
    }

    /// Just adding byte if option is Some
    pub fn arg_option(mut self, arg: Option<&Bytes>) -> Self {
        if let Some(inner) = arg {
            self.elements.push(inner.clone());
        }
        self
    }
}

impl From<CommandBuilder> for Resp2Frame {
    fn from(builder: CommandBuilder) -> Self {
        let mut frames = Vec::with_capacity(builder.elements.len());
        for byte in builder.elements {
            frames.push(Resp2Frame::BulkString(byte));
        }

        Resp2Frame::Array(frames)
    }
}

impl From<CommandBuilder> for Resp3Frame {
    fn from(builder: CommandBuilder) -> Self {
        let mut frames = Vec::with_capacity(builder.elements.len());
        for byte in builder.elements {
            frames.push(Resp3Frame::BlobString {
                data: byte,
                attributes: None,
            });
        }

        Resp3Frame::Array {
            data: frames,
            attributes: None,
        }
    }
}

impl From<CommandBuilder> for CustomCommand {
    fn from(builder: CommandBuilder) -> Self {
        CustomCommand::new(builder)
    }
}

/// Unification for `to_string()` of RESP2/3 frames
pub trait ToStringOption {
    fn to_string_option(&self) -> Option<String>;
}

impl ToStringOption for Resp2Frame {
    fn to_string_option(&self) -> Option<String> {
        self.to_string()
    }
}

impl ToStringOption for Resp3Frame {
    fn to_string_option(&self) -> Option<String> {
        self.to_string()
    }
}

/// Unification for null check of RESP2/3 frames
pub trait IsNullFrame {
    fn is_null_frame(&self) -> bool;
}

impl IsNullFrame for Resp2Frame {
    fn is_null_frame(&self) -> bool {
        self.is_null()
    }
}

impl IsNullFrame for Resp3Frame {
    fn is_null_frame(&self) -> bool {
        self.is_null()
    }
}

/// Unification for extracting integer value of Frames
pub trait ToInteger {
    /// Returns the inner integer value, None in case frame is not integer type
    fn to_integer(&self) -> Option<i64>;
}

impl ToInteger for Resp2Frame {
    fn to_integer(&self) -> Option<i64> {
        match self {
            Resp2Frame::Integer(number) => Some(*number),
            _ => None,
        }
    }
}

impl ToInteger for Resp3Frame {
    fn to_integer(&self) -> Option<i64> {
        match self {
            Resp3Frame::Number { data, attributes: _ } => Some(*data),
            _ => None,
        }
    }
}

/// Trait for string extraction of RESP2/3 frames
pub trait ToStringBytes {
    /// Extracts Bytes of Bulk (RESP2) or BLOB (RESP3) frames
    /// None if frame was not Bulk/BLOB string
    fn to_string_bytes(&self) -> Option<Bytes>;
}

impl ToStringBytes for Resp2Frame {
    fn to_string_bytes(&self) -> Option<Bytes> {
        match self {
            Resp2Frame::BulkString(data) => Some(data.clone()),
            _ => None,
        }
    }
}

impl ToStringBytes for Resp3Frame {
    fn to_string_bytes(&self) -> Option<Bytes> {
        match self {
            Resp3Frame::BlobString { data, attributes: _ } => Some(data.clone()),
            _ => None,
        }
    }
}
