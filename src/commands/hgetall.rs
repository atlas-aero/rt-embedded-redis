use crate::commands::auth::AuthCommand;
use crate::commands::builder::{CommandBuilder, ToBytesMap};
use crate::commands::hello::HelloCommand;
use crate::commands::{Command, ResponseTypeError};
use crate::network::protocol::Protocol;
use crate::network::{Client, CommandErrors, Future};
use alloc::collections::BTreeMap;
use bytes::Bytes;
use embedded_nal::TcpClientStack;
use embedded_time::Clock;

/// Abstraction for HGETALL command
pub struct HashGetAllCommand {
    /// Hash key
    key: Bytes,
}

impl HashGetAllCommand {
    pub fn new<K>(key: K) -> Self
    where
        Bytes: From<K>,
    {
        Self { key: key.into() }
    }
}

pub struct HashResponse {
    /// Field/Value map
    inner: BTreeMap<Bytes, Bytes>,
}

impl HashResponse {
    /// Extracts inner map
    #[allow(clippy::wrong_self_convention)]
    pub fn to_map(self) -> BTreeMap<Bytes, Bytes> {
        self.inner
    }

    /// Returns the given field as &str. Returns None in case field is missing or value has invalid UTF8 encoding
    pub fn get_str<F>(&self, field: F) -> Option<&str>
    where
        Bytes: From<F>,
    {
        let field: Bytes = field.into();

        match self.inner.get(&field) {
            None => None,
            Some(value) => match core::str::from_utf8(value) {
                Ok(value) => Some(value),
                Err(_) => None,
            },
        }
    }
}

impl<F> Command<F> for HashGetAllCommand
where
    F: From<CommandBuilder> + ToBytesMap,
{
    type Response = Option<HashResponse>;

    fn encode(&self) -> F {
        CommandBuilder::new("HGETALL").arg(&self.key).into()
    }

    fn eval_response(&self, frame: F) -> Result<Self::Response, ResponseTypeError> {
        let map = frame.to_map();

        if map.is_none() {
            return Err(ResponseTypeError {});
        }

        if map.as_ref().unwrap().is_empty() {
            return Ok(None);
        }

        Ok(Some(HashResponse { inner: map.unwrap() }))
    }
}

impl<'a, N: TcpClientStack, C: Clock, P: Protocol> Client<'a, N, C, P>
where
    AuthCommand: Command<<P as Protocol>::FrameType>,
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    /// Shorthand for [HashGetAllCommand]
    pub fn hgetall<K>(&'a self, key: K) -> Result<Future<'a, N, C, P, HashGetAllCommand>, CommandErrors>
    where
        <P as Protocol>::FrameType: ToBytesMap,
        <P as Protocol>::FrameType: From<CommandBuilder>,
        Bytes: From<K>,
    {
        self.send(HashGetAllCommand::new(key))
    }
}
