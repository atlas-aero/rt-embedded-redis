use crate::commands::auth::AuthCommand;
use crate::commands::builder::{CommandBuilder, ToStringOption};
use crate::commands::hello::HelloCommand;
use crate::commands::ping::PingCommand;
use crate::commands::Command;
use crate::network::buffer::Network;
use crate::network::client::{Client, CommandErrors};
use crate::network::handler::ConnectionError::{TcpConnectionFailed, TcpSocketError};
use crate::network::protocol::{Protocol, Resp2, Resp3};
use crate::network::response::MemoryParameters;
use alloc::string::{String, ToString};
use core::cell::RefCell;
use embedded_nal::{SocketAddr, TcpClientStack};
use embedded_time::duration::Extensions;
use embedded_time::duration::Microseconds;
use embedded_time::Clock;

/// Error handling for connection management
#[derive(Debug, Eq, PartialEq)]
pub enum ConnectionError {
    /// Unable to get a socket from network layer
    TcpSocketError,

    /// TCP Connect failed
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

/// Connection handler for Redis client
///
/// While the Client is not Send, the connection handler is.
/// The handler is designed with the approach that the creation of new clients is cheap.
/// Thus, the use of short-lived clients in concurrent applications is not a problem.
pub struct ConnectionHandler<N: TcpClientStack, P: Protocol>
where
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    /// Network details of Redis server
    remote: SocketAddr,

    /// Authentication credentials. None in case of no authentication.
    authentication: Option<Credentials>,

    /// Cached socket
    socket: Option<N::TcpSocket>,

    /// Previous authentication try failed, so socket gets closed on next connect()
    auth_failed: bool,

    /// Optional timeout
    /// Max. duration waiting for Redis responses
    timeout: Microseconds,

    /// Parameters for memory allocation
    memory: MemoryParameters,

    /// Redis protocol
    /// RESP3 requires Redis version >= 6.0
    protocol: P,

    /// Use PING command for testing connection
    use_ping: bool,

    /// Response to HELLO command, only used for RESP3
    pub(crate) hello_response: Option<<HelloCommand as Command<<P as Protocol>::FrameType>>::Response>,
}

impl<N: TcpClientStack> ConnectionHandler<N, Resp2> {
    /// Creates a new connection handler using RESP2 protocol
    pub fn resp2(remote: SocketAddr) -> ConnectionHandler<N, Resp2> {
        ConnectionHandler::new(remote, Resp2 {})
    }
}

impl<N: TcpClientStack> ConnectionHandler<N, Resp3> {
    /// Creates a new connection handler using RESP3 protocol
    pub fn resp3(remote: SocketAddr) -> ConnectionHandler<N, Resp3> {
        ConnectionHandler::new(remote, Resp3 {})
    }
}

impl<N: TcpClientStack, P: Protocol> ConnectionHandler<N, P>
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
            socket: None,
            auth_failed: false,
            timeout: 0.microseconds(),
            memory: MemoryParameters::default(),
            protocol,
            use_ping: false,
            hello_response: None,
        }
    }

    /// Returns a Redis client. Caches the connection for future reuse.
    /// The client has the same lifetime as the network reference.
    ///
    /// As the connection is cached, later calls are cheap.
    /// So a new client may be created when switching threads, RISC tasks, etc.
    ///
    /// *Authentication*
    /// Authentication is done automatically when creating a new connection. So the caller can
    /// expect a already authenticated and read2use client
    ///
    /// # Arguments
    ///
    /// * `network`: Mutable borrow of embedded-nal network stack
    /// * `clock`: Borrow of embedded-time clock
    ///
    /// returns: Result<Client<N, C, P>, ConnectionError>
    pub fn connect<'a, C: Clock>(
        &'a mut self,
        network: &'a mut N,
        clock: Option<&'a C>,
    ) -> Result<Client<'a, N, C, P>, ConnectionError> {
        // Previous socket is maybe faulty, so we are closing it here
        if self.auth_failed {
            self.disconnect(network);
        }

        // Check if cached socket is still connected
        self.test_socket(network, clock);

        // Reuse existing connection
        if self.socket.is_some() {
            return Ok(self.create_client(network, clock));
        }

        self.new_client(network, clock)
    }

    /// Creates and authenticates a new client
    fn new_client<'a, C: Clock>(
        &'a mut self,
        network: &'a mut N,
        clock: Option<&'a C>,
    ) -> Result<Client<'a, N, C, P>, ConnectionError> {
        self.connect_socket(network)?;
        let credentials = self.authentication.clone();
        let client = self.create_client(network, clock);

        match client.init(credentials) {
            Ok(response) => {
                self.hello_response = response;
                Ok(self.create_client(network, clock))
            }
            Err(error) => {
                self.auth_failed = true;
                Err(error)
            }
        }
    }

    /// Tests if the cached socket is still connected, if not it's closed
    fn test_socket<'a, C: Clock>(&'a mut self, network: &'a mut N, clock: Option<&'a C>) {
        if self.socket.is_none() {
            return;
        }

        if self.use_ping && self.ping(network, clock).is_err() {
            self.disconnect(network);
        }
    }

    /// Sends ping command for testing the socket
    fn ping<'a, C: Clock>(
        &'a mut self,
        network: &'a mut N,
        clock: Option<&'a C>,
    ) -> Result<(), CommandErrors> {
        self.create_client(network, clock).ping()?.wait()?;
        Ok(())
    }

    /// Disconnects the connection
    pub fn disconnect(&mut self, network: &mut N) {
        if self.socket.is_none() {
            return;
        }

        let _ = network.close(self.socket.take().unwrap());
        self.auth_failed = false;
    }

    /// Creates a new TCP connection
    fn connect_socket(&mut self, network: &mut N) -> Result<(), ConnectionError> {
        let socket_result = network.socket();
        if socket_result.is_err() {
            return Err(TcpSocketError);
        }

        let mut socket = socket_result.unwrap();
        if network.connect(&mut socket, self.remote).is_err() {
            let _ = network.close(socket);
            return Err(TcpConnectionFailed);
        };

        self.socket = Some(socket);
        Ok(())
    }

    /// Creates a new client instance
    fn create_client<'a, C: Clock>(
        &'a mut self,
        stack: &'a mut N,
        clock: Option<&'a C>,
    ) -> Client<'a, N, C, P> {
        Client {
            network: Network::new(
                RefCell::new(stack),
                RefCell::new(self.socket.as_mut().unwrap()),
                self.protocol.clone(),
                self.memory.clone(),
            ),
            timeout_duration: self.timeout,
            clock,
            hello_response: self.hello_response.as_ref(),
        }
    }
}

impl<N: TcpClientStack, P: Protocol> ConnectionHandler<N, P>
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

    /// Using PING command for testing connections
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
