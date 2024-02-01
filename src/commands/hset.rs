use crate::commands::auth::AuthCommand;
use crate::commands::builder::{CommandBuilder, ToInteger};
use crate::commands::hello::HelloCommand;
use crate::commands::{Command, ResponseTypeError};
use crate::network::protocol::Protocol;
use crate::network::{Client, CommandErrors, Future};
use bytes::Bytes;
use embedded_nal::TcpClientStack;
use embedded_time::Clock;

/// Abstraction of HSET command
pub struct HashSetCommand<const N: usize> {
    /// Hash key
    key: Bytes,

    /// Field/Value paris
    fields: [(Bytes, Bytes); N],
}

impl HashSetCommand<1> {
    pub fn new(key: Bytes, field: Bytes, value: Bytes) -> Self {
        Self {
            key,
            fields: [(field, value)],
        }
    }
}

impl<const N: usize> HashSetCommand<N> {
    /// Constructs a new command with multiple field/value paris
    pub fn multiple(key: Bytes, fields: [(Bytes, Bytes); N]) -> Self {
        Self { key, fields }
    }
}

impl<F: From<CommandBuilder> + ToInteger, const N: usize> Command<F> for HashSetCommand<N> {
    type Response = i64;

    fn encode(&self) -> F {
        let mut builder = CommandBuilder::new("HSET").arg(&self.key);

        for (field, value) in &self.fields {
            builder = builder.arg(field).arg(value);
        }

        builder.into()
    }

    fn eval_response(&self, frame: F) -> Result<Self::Response, ResponseTypeError> {
        frame.to_integer().ok_or(ResponseTypeError {})
    }
}

impl<'a, N: TcpClientStack, C: Clock, P: Protocol> Client<'a, N, C, P>
where
    AuthCommand: Command<<P as Protocol>::FrameType>,
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    /// Shorthand for [HashSetCommand]
    /// For setting multiple fields, use [<P as Protocol>::FrameType: From<CommandBuilder>] directly instead
    pub fn hset<K, F, V>(
        &'a self,
        key: K,
        field: F,
        value: V,
    ) -> Result<Future<'a, N, C, P, HashSetCommand<1>>, CommandErrors>
    where
        Bytes: From<K>,
        Bytes: From<F>,
        Bytes: From<V>,
        <P as Protocol>::FrameType: ToInteger,
        <P as Protocol>::FrameType: From<CommandBuilder>,
    {
        self.send(HashSetCommand::new(key.into(), field.into(), value.into()))
    }
}
