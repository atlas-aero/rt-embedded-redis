use crate::commands::hello::HelloCommand;
use crate::commands::helpers::{CmdStr, RespInt};
use crate::commands::Command;
use crate::network::buffer::Network;
use crate::network::protocol::Protocol;
use crate::network::response::MemoryParameters;
use crate::network::tests::mocks::MockTcpError::Error1;
use crate::network::Client;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::{format, vec};
use bytes::{BufMut, Bytes, BytesMut};
use core::cell::RefCell;
use embedded_nal::SocketAddr;
use embedded_nal::TcpClientStack;
use embedded_time::clock::Error;
use embedded_time::duration::{Duration, Extensions};
use embedded_time::fixed_point::FixedPoint;
use embedded_time::fraction::Fraction;
use embedded_time::timer::param::{Armed, OneShot};
use embedded_time::{Clock, Instant, Timer};
use mockall::mock;
use redis_protocol::resp2::types::Frame as Resp2Frame;
use redis_protocol::resp3::encode::complete::encode_bytes as resp3_encode_bytes;
use redis_protocol::resp3::types::{Frame as Resp3Frame, FrameMap};
use std::io::Write;

#[derive(Debug)]
pub struct SocketMock {
    pub id: i32,
}

impl SocketMock {
    pub fn new(id: i32) -> Self {
        SocketMock { id }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum MockTcpError {
    Error1,
}

mock! {
    #[derive(Debug)]
    pub NetworkStack {}

    impl TcpClientStack for NetworkStack {
        type TcpSocket = SocketMock;
        type Error = MockTcpError;

        fn socket(&mut self) -> Result<<Self as TcpClientStack>::TcpSocket, <Self as TcpClientStack>::Error>;

        fn connect(
            &mut self,
            socket: &mut SocketMock,
            remote: SocketAddr,
        ) -> nb::Result<(), <Self as TcpClientStack>::Error>;

        fn is_connected(&mut self, socket: &SocketMock) -> Result<bool, <Self as TcpClientStack>::Error>;

        fn send(
            &mut self,
            socket: &mut SocketMock,
            buffer: &[u8],
        ) -> nb::Result<usize, <Self as TcpClientStack>::Error>;

        fn receive(
            &mut self,
            socket: &mut SocketMock,
            buffer: &mut [u8],
        ) -> nb::Result<usize, <Self as TcpClientStack>::Error>;

        fn close(&mut self, socket: SocketMock) -> Result<(), <Self as TcpClientStack>::Error>;
    }
}

pub struct NetworkMockBuilder {
    stack: MockNetworkStack,
}

/// Helper for constructing network layer mock
impl NetworkMockBuilder {
    /// Simulates a error while fetching socket
    pub fn socket_error(mut self) -> Self {
        self.stack.expect_socket().times(1).returning(move || Err(Error1));
        self
    }

    /// Expects to return a socket with the given ID
    pub fn socket(mut self, socket_id: i32) -> Self {
        self.stack
            .expect_socket()
            .times(1)
            .returning(move || Ok(SocketMock::new(socket_id)));
        self
    }

    /// Asserts that connect is called
    pub fn connect(mut self, socket_id: i32) -> Self {
        self.stack.expect_connect().times(1).returning(move |socket, _| {
            assert_eq!(socket_id, socket.id);
            nb::Result::Ok(())
        });
        self
    }

    /// Simulates a TCP error while connecting
    pub fn connect_error(mut self, socket_id: i32) -> Self {
        self.stack.expect_connect().times(1).returning(move |socket, _| {
            assert_eq!(socket_id, socket.id);
            nb::Result::Err(nb::Error::Other(Error1))
        });
        self
    }

    /// Asserts that is_connected is called
    pub fn expect_is_connected(mut self, socket_id: i32, is_connected: bool) -> Self {
        self.stack.expect_is_connected().times(1).returning(move |socket| {
            assert_eq!(socket_id, socket.id);
            Ok(is_connected)
        });
        self
    }

    /// Simulates an error on is_connected call
    pub fn expect_is_connected_error(mut self, socket_id: i32) -> Self {
        self.stack.expect_is_connected().times(1).returning(move |socket| {
            assert_eq!(socket_id, socket.id);
            Err(Error1)
        });
        self
    }

    /// Asserts that close is called
    pub fn close(mut self, socket_id: i32) -> Self {
        self.stack.expect_close().times(1).returning(move |socket| {
            assert_eq!(socket_id, socket.id);
            Ok(())
        });
        self
    }

    /// Expect to send the given buffer
    pub fn send(mut self, socket_id: i32, data: &'static str) -> Self {
        self.stack.expect_send().times(1).returning(move |socket, buffer| {
            assert_eq!(socket_id, socket.id);
            if !data.is_empty() {
                assert_eq!(data.to_string(), String::from_utf8(buffer.to_vec()).unwrap());
            }

            nb::Result::Ok(0)
        });
        self
    }

    /// Asserts that HELLO frame is sent
    pub fn send_hello(mut self, socket_id: i32) -> Self {
        self.stack.expect_send().times(1).returning(move |socket, buffer| {
            assert_eq!(socket_id, socket.id);
            assert_eq!("HELLO 3\r\n", String::from_utf8(buffer.to_vec()).unwrap());
            nb::Result::Ok(0)
        });
        self
    }

    /// Prepares TCP TX error
    pub fn send_error(mut self) -> Self {
        self.stack
            .expect_send()
            .times(1)
            .returning(move |_, _| nb::Result::Err(nb::Error::Other(Error1)));
        self
    }

    /// Simulates a Redis error response
    pub fn response_error(mut self) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let _ = buffer.write(b"-Error\r\n").unwrap();
            nb::Result::Ok(8)
        });
        self
    }

    /// Simulates a TCP RX error
    pub fn receive_tcp_error(mut self) -> Self {
        self.stack
            .expect_receive()
            .times(1)
            .returning(move |_, _| nb::Result::Err(nb::Error::Other(MockTcpError::Error1)));
        self
    }

    /// Prepares network stack to respond with OK
    pub fn response_ok(mut self) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let _ = buffer.write(b"+OK\r\n").unwrap();
            nb::Result::Ok(5)
        });
        self
    }

    /// Prepares custom response data
    pub fn response(mut self, data: &'static str) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let _ = buffer.write(data.as_bytes()).unwrap();
            nb::Result::Ok(data.len())
        });
        self
    }

    /// Prepares custom response data
    pub fn response_string(mut self, data: &'static str) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let frame = format!("${}\r\n{}\r\n", data.len(), data);
            let _ = buffer.write(frame.as_bytes()).unwrap();
            nb::Result::Ok(frame.len())
        });
        self
    }

    /// Simulates a confirmed subscription
    pub fn sub_confirmation_resp3(mut self, topic: &'static str, channel_count: usize) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let frame = b">3\r\n+subscribe\r\n";
            let _ = buffer.write(frame).unwrap();
            nb::Result::Ok(frame.len())
        });

        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let frame = format!("+{}\r\n:{}\r\n", topic, channel_count);
            let _ = buffer.write(frame.as_bytes()).unwrap();
            nb::Result::Ok(frame.len())
        });

        self
    }

    /// Simulates a confirmed unsubscription
    pub fn unsub_confirmation_resp3(mut self, topic: &'static str, channel_count: usize) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let frame = b">3\r\n+unsubscribe\r\n";
            let _ = buffer.write(frame).unwrap();
            nb::Result::Ok(frame.len())
        });

        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let frame = format!("+{}\r\n:{}\r\n", topic, channel_count);
            let _ = buffer.write(frame.as_bytes()).unwrap();
            nb::Result::Ok(frame.len())
        });

        self
    }

    /// Simulates a published message
    pub fn sub_message(mut self, channel: &'static str, payload: &'static str) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let frame = b">3\r\n+message\r\n";
            let _ = buffer.write(frame).unwrap();
            nb::Result::Ok(frame.len())
        });

        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let frame = format!("+{}\r\n+{}\r\n", channel, payload);
            let _ = buffer.write(frame.as_bytes()).unwrap();
            nb::Result::Ok(frame.len())
        });

        self
    }

    /// Prepares RESP3 Null response
    #[allow(unused)]
    pub fn response_null_resp3(mut self) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let _ = buffer.write(b"_\r\n").unwrap();
            nb::Result::Ok(3)
        });
        self
    }

    /// Prepares RESP2 Null string response
    #[allow(unused)]
    pub fn response_null_resp2(mut self) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let _ = buffer.write(b"$-1\r\n").unwrap();
            nb::Result::Ok(5)
        });
        self
    }

    /// Prepares binary response
    #[allow(unused)]
    pub fn response_binary(mut self, data: &'static [u8]) -> Self {
        let mut frame = vec![b'$'];
        frame.put_slice(data.len().to_string().as_bytes());
        frame.put_slice(b"\r\n");
        frame.put_slice(data);
        frame.put_slice(b"\r\n");

        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let _ = buffer.write(&frame).unwrap();
            nb::Result::Ok(frame.len())
        });
        self
    }

    /// Simulates correct HELLO response
    pub fn response_hello(mut self) -> Self {
        let frame = MockFrames::hello();
        let mut bytes = BytesMut::new();
        resp3_encode_bytes(&mut bytes, &frame).unwrap();

        let mut byte_chunks = vec![];
        for chunk in bytes.chunks(32) {
            byte_chunks.push(Bytes::copy_from_slice(chunk));
        }

        for chunk in byte_chunks {
            self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
                let _ = buffer.write(chunk.as_ref()).unwrap();
                nb::Result::Ok(chunk.len())
            });
        }
        self
    }

    /// Simulates no pending data (in nb context => WouldBlock)
    pub fn response_no_data(mut self) -> Self {
        self.stack
            .expect_receive()
            .times(1)
            .returning(move |_, _| nb::Result::Err(nb::Error::WouldBlock));
        self
    }

    pub fn into_mock(self) -> MockNetworkStack {
        self.stack
    }
}

impl Default for NetworkMockBuilder {
    fn default() -> Self {
        Self {
            stack: MockNetworkStack::new(),
        }
    }
}

pub struct MockFrames {}

impl MockFrames {
    pub fn hello() -> Resp3Frame {
        let mut map = FrameMap::new();
        map.insert(CmdStr::new("server").to_blob(), CmdStr::new("redis").to_blob());
        map.insert(CmdStr::new("version").to_blob(), CmdStr::new("6.0.0").to_blob());
        map.insert(CmdStr::new("proto").to_blob(), RespInt::new(3).to_number());
        map.insert(CmdStr::new("id").to_blob(), RespInt::new(10).to_number());
        map.insert(CmdStr::new("mode").to_blob(), CmdStr::new("standalone").to_blob());
        map.insert(CmdStr::new("role").to_blob(), CmdStr::new("master").to_blob());
        map.insert(
            CmdStr::new("modules").to_blob(),
            Resp3Frame::Array {
                data: vec![],
                attributes: None,
            },
        );

        Resp3Frame::Map {
            data: map,
            attributes: None,
        }
    }

    pub fn ok_resp2() -> Resp2Frame {
        Resp2Frame::SimpleString(Bytes::from_static("OK".as_bytes()))
    }

    pub fn ok_resp3() -> Resp3Frame {
        Resp3Frame::SimpleString {
            data: Bytes::from_static("OK".as_bytes()),
            attributes: None,
        }
    }
}

#[derive(Debug)]
pub struct TestClock {
    pub next_instants: RefCell<Vec<u64>>,
}

impl Clock for TestClock {
    type T = u64;
    const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000_000);

    fn try_now(&self) -> Result<Instant<Self>, Error> {
        if self.next_instants.borrow().len() == 0 {
            return Err(Error::Unspecified);
        }

        Ok(Instant::new(self.next_instants.borrow_mut().remove(0)))
    }

    fn new_timer<Dur: Duration>(&self, duration: Dur) -> Timer<OneShot, Armed, Self, Dur>
    where
        Dur: FixedPoint,
    {
        Timer::new(self, duration)
    }
}

impl TestClock {
    pub fn new(next_instants: Vec<u64>) -> Self {
        TestClock {
            next_instants: RefCell::new(next_instants),
        }
    }
}

pub fn create_mocked_client<'a, P: Protocol>(
    network_stack: &'a mut MockNetworkStack,
    socket: &'a mut SocketMock,
    clock: &'a TestClock,
    protocol: P,
) -> Client<'a, MockNetworkStack, TestClock, P>
where
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    Client {
        network: Network::new(
            RefCell::new(network_stack),
            RefCell::new(socket),
            protocol,
            MemoryParameters::default(),
        ),
        timeout_duration: 0.microseconds(),
        clock: Some(clock),
        hello_response: None,
    }
}
