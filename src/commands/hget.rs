use crate::commands::auth::AuthCommand;
use crate::commands::builder::{CommandBuilder, IsNullFrame, ToStringBytes};
use crate::commands::get::GetResponse;
use crate::commands::hello::HelloCommand;
use crate::commands::{Command, ResponseTypeError};
use crate::network::protocol::Protocol;
use crate::network::{Client, CommandErrors, Future};
use bytes::Bytes;
use embedded_nal::TcpClientStack;
use embedded_time::Clock;

/// Abstraction for HGET command
pub struct HashGetCommand {
    /// Hash key
    key: Bytes,

    /// Hash field to receive
    field: Bytes,
}

impl HashGetCommand {
    pub fn new<K, F>(key: K, field: F) -> Self
    where
        Bytes: From<K>,
        Bytes: From<F>,
    {
        Self {
            key: key.into(),
            field: field.into(),
        }
    }
}

impl<F> Command<F> for HashGetCommand
where
    F: From<CommandBuilder> + IsNullFrame + ToStringBytes,
{
    type Response = Option<GetResponse>;

    fn encode(&self) -> F {
        CommandBuilder::new("HGET").arg(&self.key).arg(&self.field).into()
    }

    fn eval_response(&self, frame: F) -> Result<Self::Response, ResponseTypeError> {
        GetResponse::from_frame(frame)
    }
}

impl<'a, N: TcpClientStack, C: Clock, P: Protocol> Client<'a, N, C, P>
where
    AuthCommand: Command<<P as Protocol>::FrameType>,
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    /// Shorthand for [GetCommand]
    pub fn hget<K, F>(
        &'a self,
        key: K,
        field: F,
    ) -> Result<Future<'a, N, C, P, HashGetCommand>, CommandErrors>
    where
        <P as Protocol>::FrameType: ToStringBytes,
        <P as Protocol>::FrameType: IsNullFrame,
        <P as Protocol>::FrameType: From<CommandBuilder>,
        Bytes: From<K>,
        Bytes: From<F>,
    {
        self.send(HashGetCommand::new(key, field))
    }
}
