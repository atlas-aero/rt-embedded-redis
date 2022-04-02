use crate::commands::hello::HelloCommand;
use crate::commands::set::SetCommand;
use crate::commands::Command;
use crate::network::buffer::Network;
use crate::network::client::CommandErrors::{
    CommandResponseViolation, ErrorResponse, InvalidFuture, ProtocolViolation, TcpError, Timeout, TimerError,
};
use crate::network::client::{Client, RedisClient};
use crate::network::handler::ConnectionError::{AuthenticationError, ProtocolSwitchError};
use crate::network::handler::Credentials;
use crate::network::protocol::{Protocol, Resp2, Resp3};
use crate::network::tests::mocks::MockTcpError::Error1;
use crate::network::tests::mocks::{MockNetworkStack, NetworkMockBuilder, SocketMock, TestClock};
use alloc::string::ToString;
use alloc::vec;
use bytes::Bytes;
use core::cell::RefCell;
use embedded_time::duration::Extensions;

#[test]
fn test_resp2_init_no_authentication() {
    // By default no call to any method is expected
    let mut network = MockNetworkStack::new();
    let clock = TestClock::new(vec![]);
    let mut socket = SocketMock::new(1);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    client.init(None).unwrap();
}

#[test]
fn test_resp2_init_send_tcp_error() {
    let clock = TestClock::new(vec![]);
    let mut network = MockNetworkStack::new();

    network
        .expect_send()
        .times(1)
        .returning(move |_, _| nb::Result::Err(nb::Error::Other(Error1)));

    let mut socket = SocketMock::new(1);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let result = client.init(Some(Credentials::password_only("test")));
    assert_eq!(AuthenticationError(TcpError), result.unwrap_err());
}

#[test]
fn test_resp2_init_correct_message_sent() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "*2\r\n$4\r\nAUTH\r\n$9\r\nsecret123\r\n")
        .response_ok()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    client.init(Some(Credentials::password_only("secret123"))).unwrap();
}

#[test]
fn test_resp2_init_receive_tcp_error() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new().send(1, "").receive_tcp_error().into_mock();

    let mut socket = SocketMock::new(1);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let result = client.init(Some(Credentials::password_only("secret123")));
    assert_eq!(AuthenticationError(TcpError), result.unwrap_err());
}

#[test]
fn test_resp2_init_negative_response() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new().send(1, "").response_error().into_mock();

    let mut socket = SocketMock::new(1);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let result = client.init(Some(Credentials::password_only("secret123")));
    assert_eq!(
        AuthenticationError(ErrorResponse("Error".to_string())),
        result.unwrap_err()
    );
}

#[test]
fn test_resp2_init_response_split() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "")
        .response("+O")
        .response_no_data()
        .response("K\r\n")
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    client.init(Some(Credentials::password_only("secret123"))).unwrap();
}

#[test]
fn test_resp3_init_not_auth_just_hello() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new().send_hello(164).response_hello().into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp3 {});

    client.init(None).unwrap();
}

#[test]
fn test_resp3_init_auth_password_only() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "*2\r\n$4\r\nAUTH\r\n$9\r\nsecret123\r\n")
        .send_hello(164)
        .response_ok()
        .response_hello()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp3 {});

    client.init(Some(Credentials::password_only("secret123"))).unwrap();
}

#[test]
fn test_resp3_init_auth_acl() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "*3\r\n$4\r\nAUTH\r\n$6\r\nuser01\r\n$9\r\nsecret123\r\n")
        .send_hello(164)
        .response_ok()
        .response_hello()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp3 {});

    client.init(Some(Credentials::acl("user01", "secret123"))).unwrap();
}

#[test]
fn test_resp3_init_auth_failure() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new().send(164, "").response_error().into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp3 {});

    let result = client.init(Some(Credentials::acl("user01", "secret123")));
    assert_eq!(
        AuthenticationError(ErrorResponse("Error".to_string())),
        result.unwrap_err()
    )
}

#[test]
fn test_resp3_init_hello_tcp_tx_error() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new().send_error().into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp3 {});

    let result = client.init(None);
    assert_eq!(ProtocolSwitchError(TcpError), result.unwrap_err())
}

#[test]
fn test_resp3_init_hello_tcp_rx_error() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new().send(164, "").receive_tcp_error().into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp3 {});

    let result = client.init(None);
    assert_eq!(ProtocolSwitchError(TcpError), result.unwrap_err())
}

#[test]
fn test_resp3_init_hello_error_response() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new().send(164, "").response_error().into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp3 {});

    let result = client.init(None);
    assert_eq!(
        ProtocolSwitchError(ErrorResponse("Error".to_string())),
        result.unwrap_err()
    )
}

#[test]
fn test_timeout_expired() {
    let clock = TestClock::new(vec![
        100, // Timer creation
        200, // First receive() call
        300, // Second receive() call
    ]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "")
        .response_no_data()
        .response_no_data()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = Client {
        network: Network::new(RefCell::new(&mut network), RefCell::new(&mut socket), Resp2 {}),
        timeout_duration: 150.microseconds(),
        clock: Some(&clock),
        hello_response: None,
    };

    let result = client.init(Some(Credentials::password_only("secret123")));
    assert_eq!(AuthenticationError(Timeout), result.unwrap_err())
}

#[test]
fn test_timeout_timer_error() {
    let clock = TestClock::new(vec![
        100, // Timer creation
        200, // First receive() call
    ]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "")
        .response_no_data()
        .response_no_data()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = Client {
        network: Network::new(RefCell::new(&mut network), RefCell::new(&mut socket), Resp2 {}),
        timeout_duration: 150.microseconds(),
        clock: Some(&clock),
        hello_response: None,
    };

    let result = client.init(Some(Credentials::password_only("secret123")));
    assert_eq!(AuthenticationError(TimerError), result.unwrap_err())
}

#[test]
fn test_timeout_not_expired() {
    let clock = TestClock::new(vec![
        100, // Timer creation
        200, // First receive() call
        300, // Second receive() call
    ]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "")
        .response_no_data()
        .response_no_data()
        .response_ok()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = Client {
        network: Network::new(RefCell::new(&mut network), RefCell::new(&mut socket), Resp2 {}),
        timeout_duration: 250.microseconds(),
        clock: Some(&clock),
        hello_response: None,
    };

    client.init(Some(Credentials::password_only("secret123"))).unwrap();
}

#[test]
fn test_set_ok_response() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "*3\r\n$3\r\nSET\r\n$8\r\ntest_key\r\n$4\r\ntest\r\n")
        .response_no_data()
        .response_ok()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let response = client.send(SetCommand::new("test_key", "test")).unwrap().wait().unwrap();
    assert_eq!((), response);
}

#[test]
fn test_set_error_response() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new().send(164, "").response_error().into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let result = client.send(SetCommand::new("test_key", "test")).unwrap().wait().unwrap_err();
    assert_eq!(ErrorResponse("Error".to_string()), result);
}

#[test]
fn test_set_unknown_response() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new().send(164, "").response("+UNKNOWN\r\n").into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let response = client.send(SetCommand::new("test_key", "test")).unwrap().wait();
    assert_eq!(CommandResponseViolation, response.unwrap_err());
}

#[test]
fn test_faulty_response() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new().send(164, "").response("UNDEFINED\r\n").into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let result = client.send(SetCommand::new("test_key", "test")).unwrap().wait();
    assert_eq!(ProtocolViolation, result.unwrap_err())
}

#[test]
fn test_future_ready_true() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new().send(164, "").response_ok().into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let mut future = client.send(SetCommand::new("first", "future")).unwrap();

    assert!(future.ready());
    assert_eq!((), future.wait().unwrap());
}

#[test]
fn test_future_not_ready_no_data_received() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new().send(164, "").response_no_data().into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let mut future = client.send(SetCommand::new("first", "future")).unwrap();
    assert!(!future.ready());
}

#[test]
fn test_future_not_ready_incomplete_frame() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "")
        .response("+O")
        .response_no_data()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let mut future = client.send(SetCommand::new("first", "future")).unwrap();

    assert!(!future.ready());
}

#[test]
fn test_future_ready_error() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new().send(164, "").receive_tcp_error().into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let mut future = client.send(SetCommand::new("first", "future")).unwrap();

    assert!(future.ready());
    assert_eq!(TcpError, future.wait().unwrap_err());
}

#[test]
/// Tests asserts if futures are called in sequence
fn test_multiple_responses_future_wait_in_order() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "")
        .send(164, "")
        .response_error()
        .response("+O")
        .response_no_data()
        .response("K\r\n")
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let first = client.send(SetCommand::new("first", "future")).unwrap();
    let second = client.send(SetCommand::new("second", "future")).unwrap();

    assert_eq!(ErrorResponse("Error".to_string()), first.wait().unwrap_err());
    assert_eq!((), second.wait().unwrap());
}

#[test]
/// Tests asserts if futures are called out of order
fn test_multiple_responses_future_wait_crossed() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "")
        .send(164, "")
        .response_error()
        .response("+O")
        .response_no_data()
        .response("K\r\n")
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let first = client.send(SetCommand::new("first", "future")).unwrap();
    let second = client.send(SetCommand::new("second", "future")).unwrap();

    assert_eq!((), second.wait().unwrap());
    assert_eq!(ErrorResponse("Error".to_string()), first.wait().unwrap_err());
}

#[test]
fn test_multiple_responses_partly_complete() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "")
        .send(164, "")
        .response_ok()
        .response("+O")
        .response_no_data()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let mut first = client.send(SetCommand::new("first", "future")).unwrap();
    let mut second = client.send(SetCommand::new("second", "future")).unwrap();

    assert!(first.ready());
    assert!(!second.ready());
    assert_eq!((), first.wait().unwrap());
}

#[test]
fn test_futures_invalidated_on_timeout() {
    let clock = TestClock::new(vec![
        100, // Timer creation
        101, // Timer creation
        200, // First receive() call
        300, // Second receive() call
    ]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "")
        .send(164, "")
        .response_no_data()
        .response_no_data()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = Client {
        network: Network::new(RefCell::new(&mut network), RefCell::new(&mut socket), Resp2 {}),
        timeout_duration: 150.microseconds(),
        clock: Some(&clock),
        hello_response: None,
    };

    let first = client.send(SetCommand::new("timeout", "future")).unwrap();
    let second = client.send(SetCommand::new("second", "future")).unwrap();
    assert_eq!(Timeout, first.wait().unwrap_err());
    assert_eq!(InvalidFuture, second.wait().unwrap_err());
}

#[test]
fn test_future_invalidated_on_faulty_response() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "")
        .send(164, "")
        .send(164, "")
        .response("_faulty\r\n")
        .response("more faulty data")
        .response_no_data()
        .response_ok()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let first = client.send(SetCommand::new("faulty", "future")).unwrap();
    let second = client.send(SetCommand::new("second", "future")).unwrap();

    assert_eq!(ProtocolViolation, first.wait().unwrap_err());
    assert_eq!(InvalidFuture, second.wait().unwrap_err());

    let third = client.send(SetCommand::new("third", "future")).unwrap();
    assert_eq!((), third.wait().unwrap());
}

/// Tests dropped future, which wait() method was not called.
/// Response data of this futures is handled at next send() call
/// In the following scenario the data arrives at the next send call
#[test]
fn test_future_dropped_received_at_send() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "")
        .send(164, "")
        .response_ok()
        .response_no_data()
        .response_ok()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    {
        let _ = client.send(SetCommand::new("key", "value"));
    }

    assert_eq!(1, client.network.get_dropped_future_count());
    let future = client.send(SetCommand::new("key", "value")).unwrap();
    assert_eq!(0, client.network.get_dropped_future_count());
    assert_eq!(0, client.network.get_pending_frame_count());
    future.wait().unwrap();
}

/// Tests dropped future, which wait() method was not called.
/// Response data of this futures is handled at next send() call
/// In the following scenario the data arrives at the next future wait() call
#[test]
fn test_future_dropped_received_at_next_future() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "")
        .send(164, "")
        .send(164, "")
        .response_no_data() // Called at second send, no data arrived yet
        .response_ok() // Data of first (dropped) future
        .response_ok() // Data of second future, which wait() method is called
        .response_no_data() // Called a third send, no more data to receive
        .response_ok() // Data of third future
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    {
        let _ = client.send(SetCommand::new("key", "value"));
    }

    assert_eq!(1, client.network.get_dropped_future_count());
    let second = client.send(SetCommand::new("key", "value")).unwrap();
    // Data of dropped future is not arrived yet
    assert_eq!(1, client.network.get_dropped_future_count());
    assert_eq!(0, client.network.get_pending_frame_count());

    // Data of dropped future arrives now
    second.wait().unwrap();
    assert_eq!(1, client.network.get_pending_frame_count());

    // Data of dropped future gets cleared
    assert_eq!(1, client.network.get_dropped_future_count());
    let third = client.send(SetCommand::new("key", "value")).unwrap();
    assert_eq!(0, client.network.get_dropped_future_count());
    assert_eq!(0, client.network.get_pending_frame_count());

    third.wait().unwrap();
}

/// Tests dropped future, which wait() method was not called.
/// Response data of this futures is handled at next send() call
/// In the following scenario a fatal error occured, so the dropped future got invalidated in the
/// meanwhile
#[test]
fn test_future_dropped_invalidated() {
    let clock = TestClock::new(vec![
        100, // Timer creation of first future
        101, // Timer creation of second future
        200, // First receive() call of first future
        300, // Second receive() call of first future <-- Timeout threshold is reached here
        400, // Timer creation of third future
        450, // Receive() call of third future
    ]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "")
        .send(164, "")
        .send(164, "")
        .response_no_data() // First and second call during timeout
        .response_no_data()
        .response_no_data() // Third call during socket clearance caused by timeout
        .response_no_data() // Fourth call during "dropped-future handler"
        .response_ok()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = Client {
        network: Network::new(RefCell::new(&mut network), RefCell::new(&mut socket), Resp2 {}),
        timeout_duration: 150.microseconds(),
        clock: Some(&clock),
        hello_response: None,
    };

    let first = client.send(SetCommand::new("timeout", "future")).unwrap();
    {
        let _second = client.send(SetCommand::new("second", "future")).unwrap();
    }
    assert_eq!(Timeout, first.wait().unwrap_err());

    // Second future is invalidated, so just removed from the dropped future list
    assert_eq!(1, client.network.get_dropped_future_count());
    let third = client.send(SetCommand::new("key", "value")).unwrap();
    assert_eq!(0, client.network.get_dropped_future_count());

    third.wait().unwrap();
    assert_eq!(0, client.network.get_pending_frame_count());
}

fn create_client<'a, P: Protocol>(
    network_stack: &'a mut MockNetworkStack,
    socket: &'a mut SocketMock,
    clock: &'a TestClock,
    protocol: P,
) -> Client<'a, MockNetworkStack, TestClock, P>
where
    HelloCommand: Command<<P as Protocol>::FrameType>,
{
    Client {
        network: Network::new(RefCell::new(network_stack), RefCell::new(socket), protocol),
        timeout_duration: 0.microseconds(),
        clock: Some(&clock),
        hello_response: None,
    }
}

#[test]
fn test_shorthand_get_str_argument() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n")
        .response_string()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    assert_eq!(
        "test_response",
        client.get("key").unwrap().wait().unwrap().unwrap().as_str().unwrap()
    );
}

#[test]
fn test_shorthand_get_string_argument() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "*2\r\n$3\r\nGET\r\n$8\r\ntest_key\r\n")
        .response_string()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let response = client.get("test_key".to_string()).unwrap().wait();
    assert_eq!("test_response", response.unwrap().unwrap().as_str().unwrap());
}

#[test]
fn test_shorthand_get_bytes_argument() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "*2\r\n$3\r\nGET\r\n$8\r\ntest_key\r\n")
        .response_string()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let response = client.get(Bytes::from_static(b"test_key")).unwrap().wait();
    assert_eq!("test_response", response.unwrap().unwrap().as_str().unwrap());
}

#[test]
fn test_shorthand_set_str_argument() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n")
        .response_ok()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let response = client.set("key", "value").unwrap().wait().unwrap();
    assert_eq!((), response);
}

#[test]
fn test_shorthand_set_string_argument() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n")
        .response_ok()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let response = client.set("key".to_string(), "value".to_string()).unwrap().wait().unwrap();
    assert_eq!((), response);
}

#[test]
fn test_shorthand_set_bytes_argument() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n")
        .response_ok()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let key = Bytes::from_static(b"key");
    let value = Bytes::from_static(b"value");
    let response = client.set(key, value).unwrap().wait().unwrap();
    assert_eq!((), response);
}

#[test]
fn test_shorthand_publish() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::new()
        .send(164, "*3\r\n$7\r\nPUBLISH\r\n$6\r\ncolors\r\n$6\r\norange\r\n")
        .response(":3\r\n")
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = create_client(&mut network, &mut socket, &clock, Resp2 {});

    let response = client.publish("colors", "orange").unwrap().wait().unwrap();
    assert_eq!(3, response);
}
