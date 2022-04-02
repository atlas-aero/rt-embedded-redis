//! Abstraction of HELLO command.
//!
//! For general information about this command, see the [Redis documentation](<https://redis.io/commands/hello/>).
//!
//! *As this command is executed automatically during connection initialization, there is usually no
//! need for manual execution.
//! Response of HELLO command may be retrieved from [Client](crate::network::Client#method.get_hello_response)*
//! # Basic usage
//! **Requires RESP3 protocol usage, panics on RESP2**
//!
//! Response is mapped to [HelloResponse].
//! ```
//!# use core::str::FromStr;
//!# use embedded_nal::SocketAddr;
//!# use std_embedded_nal::Stack;
//!# use std_embedded_time::StandardClock;
//!# use embedded_redis::commands::hello::HelloCommand;
//!# use embedded_redis::network::{ConnectionHandler, RedisConnectionHandler};
//!#
//! let mut stack = Stack::default();
//! let clock = StandardClock::default();
//!
//! // RESP3 protocol is essential
//! let mut connection_handler = ConnectionHandler::resp3(SocketAddr::from_str("127.0.0.1:6379").unwrap());
//! let client = connection_handler.connect(&mut stack, Some(&clock)).unwrap();
//!
//! let command = HelloCommand{};
//! let response = client.send(command).unwrap().wait().unwrap();
//!
//! assert_eq!("redis", response.server);
//! assert_eq!("master", response.role);
//! ```
use crate::commands::helpers::{CmdStr, RespMap};
use crate::commands::Command;
use alloc::string::String;
use alloc::vec::Vec;
use redis_protocol::resp2::types::Frame as Resp2Frame;
use redis_protocol::resp3::types::{Frame as Resp3Frame, RespVersion};

/// Abstraction of HELLO command.
pub struct HelloCommand {}

impl Command<Resp3Frame> for HelloCommand {
    type Response = HelloResponse;

    fn encode(&self) -> Resp3Frame {
        Resp3Frame::Hello {
            version: RespVersion::RESP3,
            auth: None,
        }
    }

    fn eval_response(&self, frame: Resp3Frame) -> Result<Self::Response, ()> {
        HelloResponse::try_from(frame)
    }
}

impl Command<Resp2Frame> for HelloCommand {
    type Response = HelloResponse;

    fn encode(&self) -> Resp2Frame {
        unimplemented!("Command requires RESP3");
    }

    fn eval_response(&self, _frame: Resp2Frame) -> Result<Self::Response, ()> {
        unimplemented!("Command requires RESP3");
    }
}

/// Mapped response to HELLO command
#[derive(Debug)]
pub struct HelloResponse {
    pub server: String,
    pub version: String,
    pub protocol: i64,
    pub id: i64,
    pub mode: String,
    pub role: String,
    pub modules: Vec<Resp3Frame>,
}

impl TryFrom<Resp3Frame> for HelloResponse {
    type Error = ();

    fn try_from(frame: Resp3Frame) -> Result<Self, Self::Error> {
        let map = match frame {
            Resp3Frame::Map { data, attributes: _ } => data,
            _ => return Err(()),
        };

        let map_cmd = RespMap::new(&map);

        Ok(HelloResponse {
            server: map_cmd.find_string("server").ok_or(())?,
            version: map_cmd.find_string("version").ok_or(())?,
            protocol: map_cmd.find_integer("proto").ok_or(())?,
            id: map_cmd.find_integer("id").ok_or(())?,
            mode: map_cmd.find_string("mode").ok_or(())?,
            role: map_cmd.find_string("role").ok_or(())?,
            modules: match map.get(&CmdStr::new("modules").to_blob()).ok_or(())? {
                Resp3Frame::Array { data, attributes: _ } => data.clone(),
                _ => return Err(()),
            },
        })
    }
}
