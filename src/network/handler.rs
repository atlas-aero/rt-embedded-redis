use crate::commands::auth::AuthCommand;
use crate::commands::builder::{CommandBuilder, ToStringOption};
use crate::commands::hello::HelloCommand;
use crate::commands::ping::PingCommand;
use crate::commands::Command;
use crate::network::buffer::Network;
use crate::network::client::{Client, CommandErrors};
use crate::network::handler::ConnectionError::TcpConnectionFailed;
use crate::network::protocol::{Protocol, Resp2, Resp3};
use crate::network::response::MemoryParameters;
use alloc::string::{String, ToString};
use core::marker::PhantomData;
use core::net::SocketAddr;
use embedded_nal_async::TcpConnect;
use embedded_time::duration::Extensions;
use embedded_time::duration::Microseconds;
use embedded_time::Clock;

/// Error handling for connection management
#[derive(Debug, Eq, PartialEq)]
pub enum ConnectionError {
    /// TCP connection failed
    TcpConnectionFailed,

    /// Authentication failed with the given sub error
    AuthenticationError(CommandErrors),

    /// Protocol switch (switch to RESP3) failed with the given sub error
    ProtocolSwitchError(CommandErrors),
}

/// Authentication credentials
#[derive(Clone)]
pub struct Credentials {
    pub(crate) username: Option<String>,
    pub(crate) password: String,
}

impl Credentials {
    /// Uses ACL based authentication
    /// Required Redis version >= 6 + ACL enabled
    pub fn acl(username: &str, password: &str) -> Self {
        Credentials {
            username: Some(username.to_string()),
            password: password.to_string(),
        }
    }

    /// Uses password-only authentication.
    /// This form just authenticates against the password set with requirepass (Redis server conf)
    pub fn password_only(password: &str) -> Self {
        Self {
            username: None,
            password: password.to_string(),
        }
    }
}

/// Configuration and connection factory for Redis clients.
///
/// Each call to [`connect`](Self::connect) creates a new connection. The returned client owns the
/// connection, which is closed by the `embedded-nal-async` implementation when it is dropped.
pub struct ConnectionHandler<N: TcpConnect, P: Protocol>
where
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    /// Network details of the Redis server.
    remote: SocketAddr,

    /// Authentication credentials. `None` if authentication is disabled.
    authentication: Option<Credentials>,

    /// Maximum duration to wait for Redis responses.
    timeout: Microseconds,

    /// Parameters controlling response-buffer memory allocation.
    memory: MemoryParameters,

    /// Redis protocol implementation. RESP3 requires Redis 6.0 or newer.
    protocol: P,

    /// Whether newly opened connections are verified with a PING command.
    use_ping: bool,

    /// Associates the handler with its asynchronous network stack type without owning the stack.
    network: PhantomData<N>,
}

impl<N: TcpConnect> ConnectionHandler<N, Resp2> {
    /// Creates a new connection handler using RESP2 protocol
    pub fn resp2(remote: SocketAddr) -> Self {
        ConnectionHandler::new(remote, Resp2 {})
    }
}

impl<N: TcpConnect> ConnectionHandler<N, Resp3> {
    /// Creates a new connection handler using RESP3 protocol
    pub fn resp3(remote: SocketAddr) -> Self {
        ConnectionHandler::new(remote, Resp3 {})
    }
}

impl<N: TcpConnect, P: Protocol> ConnectionHandler<N, P>
where
    AuthCommand: Command<<P as Protocol>::FrameType>,
    HelloCommand: Command<<P as Protocol>::FrameType>,
    PingCommand: Command<<P as Protocol>::FrameType>,
    <P as Protocol>::FrameType: ToStringOption,
    <P as Protocol>::FrameType: From<CommandBuilder>,
{
    fn new(remote: SocketAddr, protocol: P) -> Self {
        ConnectionHandler {
            remote,
            authentication: None,
            timeout: 0.microseconds(),
            memory: MemoryParameters::default(),
            protocol,
            use_ping: false,
            network: PhantomData,
        }
    }

    /// Opens, authenticates and returns a Redis client.
    pub async fn connect<'a, C: Clock>(
        &self,
        network: &'a N,
        clock: Option<&'a C>,
    ) -> Result<Client<'a, N::Connection<'a>, C, P>, ConnectionError> {
        let connection = network.connect(self.remote).await.map_err(|_| TcpConnectionFailed)?;

        let mut client = Client {
            network: Network::new(connection, self.protocol.clone(), self.memory.clone()),
            timeout_duration: self.timeout,
            clock,
            hello_response: None,
        };

        client.hello_response = client.init(self.authentication.clone()).await?;

        if self.use_ping {
            client
                .ping()
                .await
                .map_err(|_| TcpConnectionFailed)?
                .wait()
                .await
                .map_err(|_| TcpConnectionFailed)?;
        }

        Ok(client)
    }
}

impl<N: TcpConnect, P: Protocol> ConnectionHandler<N, P>
where
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    /// Sets the max. duration waiting for Redis responses
    pub fn timeout(&mut self, timeout: Microseconds) -> &mut Self {
        self.timeout = timeout;
        self
    }

    /// Sets the authentication credentials
    pub fn auth(&mut self, credentials: Credentials) -> &mut Self {
        self.authentication = Some(credentials);
        self
    }

    /// Uses a PING command to verify newly created connections
    pub fn use_ping(&mut self) -> &mut Self {
        self.use_ping = true;
        self
    }

    /// Sets memory allocation parameters
    pub fn memory(&mut self, parameters: MemoryParameters) -> &mut Self {
        self.memory = parameters;
        self
    }
}
