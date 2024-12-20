//! Abstraction of AUTH command.
//!
//! For general information about this command, see the [Redis documentation](<https://redis.io/commands/auth/>).
//!
//! *Authentication is done automatically by [ConnectionHandler](crate::network::ConnectionHandler), so there is usually no need for manual execution.*
//!
//! # Password-only
//! ```
//!# use core::str::FromStr;
//!# use std::str::Bytes;
//!# use core::net::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::auth::AuthCommand;
//!# use embedded_redis::network::{ConnectionHandler, Credentials};
//!#
//! let mut stack = Stack::default();
//! let clock = StandardClock::default();
//!
//! let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//! let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!
//! // Cast from Credentials
//! let command = AuthCommand::from(&Credentials::password_only("secret123!"));
//! let _ = client.send(command);
//!
//! // Directly creating Auth command:
//! let command = AuthCommand::new(None as Option<&str>, "secret123!");
//! let _ = client.send(command);
//! ```
//! # Username/Password (ACL based authentication)
//! *Requires Redis version > 6.0 + serverside ACL configuration*
//! ```
//!# use core::str::FromStr;
//!# use core::net::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::auth::AuthCommand;
//!# use embedded_redis::network::{ConnectionHandler, Credentials};
//!#
//! let mut stack = Stack::default();
//! let clock = StandardClock::default();
//!
//! let mut connection_handler = ConnectionHandler::resp3(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//! let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!
//! // Cast from Credentials
//! let command = AuthCommand::from(&Credentials::acl("user01", "secret123!"));
//! let _ = client.send(command);
//!
//! // Directly creating Auth command:
//! let command = AuthCommand::new(Some("user01"), "secret123!");
//! let _ = client.send(command);
//! ```
//! # Error handling
//! Successful execution is terminated by returning `Ok(())` response.
//!
//! Authentication errors are normally signalled by Redis with an error response, which is mapped
//! to [CommandErrors::ErrorResponse](crate::network::CommandErrors::ErrorResponse).
//! ```
//!# use core::str::FromStr;
//!# use core::net::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::auth::AuthCommand;
//!# use embedded_redis::network::CommandErrors;
//!# use embedded_redis::network::{ConnectionHandler, Credentials};
//!#
//!# let mut stack = Stack::default();
//!# let clock = StandardClock::default();
//!#
//!# let mut connection_handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//!# let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!#
//!# let error_string = "ERR AUTH <password> called without any password configured for the default user. Are you sure your configuration is correct?".to_string();
//! let command = AuthCommand::from(&Credentials::password_only("wrong_password"));
//! let result = client.send(command).unwrap().wait().unwrap_err();
//! assert_eq!(CommandErrors::ErrorResponse(error_string), result);
//! ```

use crate::commands::builder::{CommandBuilder, ToStringOption};
use crate::commands::{Command, ResponseTypeError};
use crate::network::handler::Credentials;
use bytes::Bytes;

pub struct AuthCommand {
    /// Optionally sets a username for ACL based authentication, which requires
    /// Redis version >= 6 + ACL enabled
    username: Option<Bytes>,
    password: Bytes,
}

impl AuthCommand {
    pub fn new<U, P>(username: Option<U>, password: P) -> Self
    where
        U: Into<Bytes>,
        P: Into<Bytes>,
    {
        let mut user_bytes = None;
        if let Some(bytes) = username {
            user_bytes = Some(bytes.into());
        }

        AuthCommand {
            username: user_bytes,
            password: password.into(),
        }
    }
}

impl<F> Command<F> for AuthCommand
where
    F: ToStringOption + From<CommandBuilder>,
{
    type Response = ();

    fn encode(&self) -> F {
        CommandBuilder::new("AUTH")
            .arg_option(self.username.as_ref())
            .arg(&self.password)
            .into()
    }

    fn eval_response(&self, frame: F) -> Result<Self::Response, ResponseTypeError> {
        if frame.to_string_option().ok_or(ResponseTypeError {})? != "OK" {
            return Err(ResponseTypeError {});
        }

        Ok(())
    }
}

impl From<&Credentials> for AuthCommand {
    fn from(credentials: &Credentials) -> AuthCommand {
        AuthCommand::new(credentials.username.clone(), credentials.password.clone())
    }
}
