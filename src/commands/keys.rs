//! Abstraction of the Redis `KEYS` command.
//!
//! For general information about this command, see the
//! [Redis documentation](https://redis.io/commands/keys/).
//!
//! `KEYS` searches the currently selected Redis database for keys matching
//! the supplied glob-style pattern.

use crate::commands::auth::AuthCommand;
use crate::commands::builder::CommandBuilder;
use crate::commands::builder::ToBytesVec;
use crate::commands::hello::HelloCommand;
use crate::commands::{Command, ResponseTypeError};
use crate::network::client::{Client, CommandErrors};
use crate::network::future::Future;
use crate::network::protocol::Protocol;
use alloc::vec::Vec;
use bytes::Bytes;
use embedded_nal::TcpClientStack;
use embedded_time::Clock;

/// Abstraction for KEYS command
pub struct KeysCommand {
    pattern: Bytes,
}

impl KeysCommand {
    /// Creates a `KEYS` command with the supplied matching pattern.
    pub fn new<P>(pattern: P) -> Self
    where
        Bytes: From<P>,
    {
        Self {
            pattern: pattern.into(),
        }
    }

    /// Creates a `KEYS` command from a static string without copying it.
    pub fn static_pattern(pattern: &'static str) -> Self {
        Self {
            pattern: Bytes::from_static(pattern.as_bytes()),
        }
    }
}

/// Response returned by the Redis `KEYS` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeysResponse {
    keys: Vec<Bytes>,
}

impl KeysResponse {
    /// Creates a response from its underlying key collection.
    pub fn new(keys: Vec<Bytes>) -> Self {
        Self { keys }
    }

    /// Returns an iterator over the keys.
    pub fn iter(&self) -> impl Iterator<Item = &Bytes> {
        self.keys.iter()
    }

    /// Returns the keys as a slice.
    pub fn as_slice(&self) -> &[Bytes] {
        &self.keys
    }

    /// Extracts the underlying vector.
    pub fn to_bytes(self) -> Vec<Bytes> {
        self.keys
    }

    fn from_frame<F>(frame: F) -> Result<Self, ResponseTypeError>
    where
        F: ToBytesVec,
    {
        let keys = frame.to_bytes_vec().ok_or(ResponseTypeError {})?;
        Ok(Self::new(keys))
    }
}

impl<F> Command<F> for KeysCommand
where
    F: From<CommandBuilder> + ToBytesVec,
{
    type Response = KeysResponse;

    fn encode(&self) -> F {
        CommandBuilder::new("KEYS").arg(&self.pattern).into()
    }

    fn eval_response(&self, frame: F) -> Result<Self::Response, ResponseTypeError> {
        KeysResponse::from_frame(frame)
    }
}

impl<'a, N: TcpClientStack, C: Clock, P: Protocol> Client<'a, N, C, P>
where
    AuthCommand: Command<<P as Protocol>::FrameType>,
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    /// Shorthand for [KeysCommand]
    pub fn keys<K>(&'a self, pattern: K) -> Result<Future<'a, N, C, P, KeysCommand>, CommandErrors>
    where
        <P as Protocol>::FrameType: From<CommandBuilder> + ToBytesVec,
        Bytes: From<K>,
    {
        self.send(KeysCommand::new(pattern))
    }
}
