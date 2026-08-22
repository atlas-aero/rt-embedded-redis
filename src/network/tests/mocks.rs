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
use core::future::poll_fn;
use core::net::SocketAddr;
use core::task::Poll;
use embedded_io_async::{Error as TcpError, ErrorKind as TcpErrorKind, ErrorType};
use embedded_nal_async::TcpConnect;
use embedded_time::clock::Error;
use embedded_time::duration::{Duration, Extensions};
use embedded_time::fixed_point::FixedPoint;
use embedded_time::fraction::Fraction;
use embedded_time::timer::param::{Armed, OneShot};
use embedded_time::{Clock, Instant, Timer};
use mockall::mock;
use redis_protocol::error::RedisProtocolErrorKind::BufferTooSmall;
use redis_protocol::resp2::types::BytesFrame as Resp2Frame;
use redis_protocol::resp3::encode::complete::encode_bytes as resp3_encode_bytes;
use redis_protocol::resp3::types::{BytesFrame as Resp3Frame, FrameMap};
use std::io::Write as _;

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
    PendingRead,
}

impl TcpError for MockTcpError {
    fn kind(&self) -> TcpErrorKind {
        TcpErrorKind::Other
    }
}

impl core::fmt::Display for MockTcpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("mock TCP error")
    }
}

impl core::error::Error for MockTcpError {}

mock! {
    #[derive(Debug)]
    pub NetworkStack {
        pub fn socket(&self) -> Result<SocketMock, MockTcpError>;

        pub fn connect_socket(
            &self,
            socket: &mut SocketMock,
            remote: SocketAddr,
        ) -> Result<(), MockTcpError>;

        pub fn send(
            &self,
            socket: &mut SocketMock,
            buffer: &[u8],
        ) -> Result<usize, MockTcpError>;

        pub fn receive(
            &self,
            socket: &mut SocketMock,
            buffer: &mut [u8],
        ) -> Result<usize, MockTcpError>;

        pub fn close(&self, socket: SocketMock) -> Result<(), MockTcpError>;
    }
}

#[derive(Debug)]
pub struct MockConnection<'a> {
    stack: &'a MockNetworkStack,
    socket: Option<SocketMock>,
}

#[cfg(test)]
impl MockNetworkStack {
    pub(crate) fn connection(&self, socket: SocketMock) -> MockConnection<'_> {
        MockConnection {
            stack: self,
            socket: Some(socket),
        }
    }
}

impl ErrorType for MockConnection<'_> {
    type Error = MockTcpError;
}

impl embedded_io_async::Read for MockConnection<'_> {
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        poll_fn(
            |context| match self.stack.receive(self.socket.as_mut().unwrap(), buffer) {
                Err(MockTcpError::PendingRead) => {
                    context.waker().wake_by_ref();
                    Poll::Pending
                }
                result => Poll::Ready(result),
            },
        )
        .await
    }
}

impl embedded_io_async::Write for MockConnection<'_> {
    async fn write(&mut self, buffer: &[u8]) -> Result<usize, Self::Error> {
        self.stack.send(self.socket.as_mut().unwrap(), buffer)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl TcpConnect for MockNetworkStack {
    type Error = MockTcpError;
    type Connection<'a> = MockConnection<'a>;

    async fn connect<'a>(&'a self, remote: SocketAddr) -> Result<Self::Connection<'a>, Self::Error> {
        let mut socket = self.socket()?;
        self.connect_socket(&mut socket, remote)?;
        Ok(MockConnection {
            stack: self,
            socket: Some(socket),
        })
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
        self.stack.expect_connect_socket().times(1).returning(move |socket, _| {
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

            Ok(buffer.len())
        });
        self
    }

    /// Asserts that HELLO frame is sent
    pub fn send_hello(mut self, socket_id: i32) -> Self {
        self.stack.expect_send().times(1).returning(move |socket, buffer| {
            assert_eq!(socket_id, socket.id);
            assert_eq!("HELLO 3\r\n", String::from_utf8(buffer.to_vec()).unwrap());
            Ok(buffer.len())
        });
        self
    }

    /// Prepares TCP TX error
    pub fn send_error(mut self) -> Self {
        self.stack.expect_send().times(1).returning(move |_, _| Err(Error1));
        self
    }

    /// Simulates a Redis error response
    pub fn response_error(mut self) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let _ = buffer.write(b"-Error\r\n").unwrap();
            Ok(8)
        });
        self
    }

    /// Simulates a TCP RX error
    pub fn receive_tcp_error(mut self) -> Self {
        self.stack
            .expect_receive()
            .times(1)
            .returning(move |_, _| Err(MockTcpError::Error1));
        self
    }

    /// Simulates an orderly TCP shutdown.
    pub fn response_eof(mut self) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, _| Ok(0));
        self
    }

    /// Prepares network stack to respond with OK
    pub fn response_ok(mut self) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let _ = buffer.write(b"+OK\r\n").unwrap();
            Ok(5)
        });
        self
    }

    /// Prepares custom response data
    pub fn response(mut self, data: &'static str) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let _ = buffer.write(data.as_bytes()).unwrap();
            Ok(data.len())
        });
        self
    }

    /// Prepares custom response data
    pub fn response_string(mut self, data: &'static str) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let frame = format!("${}\r\n{}\r\n", data.len(), data);
            let _ = buffer.write(frame.as_bytes()).unwrap();
            Ok(frame.len())
        });
        self
    }

    /// Simulates a confirmed subscription
    pub fn sub_confirmation_resp3(mut self, topic: &'static str, channel_count: usize) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let frame = b">3\r\n+subscribe\r\n";
            let _ = buffer.write(frame).unwrap();
            Ok(frame.len())
        });

        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let frame = format!("+{topic}\r\n:{channel_count}\r\n");
            let _ = buffer.write(frame.as_bytes()).unwrap();
            Ok(frame.len())
        });

        self
    }

    /// Simulates a confirmed unsubscription
    pub fn unsub_confirmation_resp3(mut self, topic: &'static str, channel_count: usize) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let frame = b">3\r\n+unsubscribe\r\n";
            let _ = buffer.write(frame).unwrap();
            Ok(frame.len())
        });

        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let frame = format!("+{topic}\r\n:{channel_count}\r\n");
            let _ = buffer.write(frame.as_bytes()).unwrap();
            Ok(frame.len())
        });

        self
    }

    /// Simulates a published message
    pub fn sub_message(mut self, channel: &'static str, payload: &'static str) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let frame = b">3\r\n+message\r\n";
            let _ = buffer.write(frame).unwrap();
            Ok(frame.len())
        });

        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let frame = format!("+{channel}\r\n+{payload}\r\n");
            let _ = buffer.write(frame.as_bytes()).unwrap();
            Ok(frame.len())
        });

        self
    }

    /// Prepares RESP3 Null response
    #[allow(unused)]
    pub fn response_null_resp3(mut self) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let _ = buffer.write(b"_\r\n").unwrap();
            Ok(3)
        });
        self
    }

    /// Prepares RESP2 Null string response
    #[allow(unused)]
    pub fn response_null_resp2(mut self) -> Self {
        self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
            let _ = buffer.write(b"$-1\r\n").unwrap();
            Ok(5)
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
            Ok(frame.len())
        });
        self
    }

    /// Simulates a incomplete binary message of the given length
    #[allow(unused)]
    pub fn response_incomplete_binary<const L: usize>(mut self) -> Self {
        let mut frame = vec![b'$'];
        frame.put_slice(L.to_string().as_bytes());
        frame.put_slice(b"\r\n");
        frame.put_slice(&[0x0; L]);

        for chunk in frame.chunks(32) {
            let chunk_copy = chunk.to_vec();

            self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
                let _ = buffer.write(&chunk_copy).unwrap();
                Ok(chunk_copy.len())
            });
        }
        self
    }

    /// Simulates correct HELLO response
    pub fn response_hello(mut self) -> Self {
        let frame = MockFrames::hello();
        let mut bytes = BytesMut::new();

        // Extend buffer if needed
        while let Err(error) = resp3_encode_bytes(&mut bytes, &frame, false) {
            if let BufferTooSmall(size) = error.kind() {
                bytes.resize(bytes.len() + *size, 0x0);
            } else {
                panic!("Encoding frame failed: {:?}", error);
            }
        }

        let mut byte_chunks = vec![];
        for chunk in bytes.chunks(32) {
            byte_chunks.push(Bytes::copy_from_slice(chunk));
        }

        for chunk in byte_chunks {
            self.stack.expect_receive().times(1).returning(move |_, mut buffer: &mut [u8]| {
                let _ = buffer.write(chunk.as_ref()).unwrap();
                Ok(chunk.len())
            });
        }
        self
    }

    /// Keeps an asynchronous read pending.
    pub fn response_no_data(mut self) -> Self {
        self.stack
            .expect_receive()
            .times(0..=1)
            .returning(move |_, _| Err(MockTcpError::PendingRead));
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
        if self.next_instants.borrow().is_empty() {
            return Err(Error::Unspecified);
        }

        Ok(Instant::new(self.next_instants.borrow_mut().remove(0)))
    }

    fn new_timer<Dur>(&self, duration: Dur) -> Timer<'_, OneShot, Armed, Self, Dur>
    where
        Dur: FixedPoint,
        Dur: Duration,
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
    network_stack: &'a MockNetworkStack,
    socket: SocketMock,
    clock: &'a TestClock,
    protocol: P,
) -> Client<'a, MockConnection<'a>, TestClock, P>
where
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    Client {
        network: Network::new(
            MockConnection {
                stack: network_stack,
                socket: Some(socket),
            },
            protocol,
            MemoryParameters::default(),
        ),
        timeout_duration: 0.microseconds(),
        clock: Some(clock),
        hello_response: None,
    }
}
