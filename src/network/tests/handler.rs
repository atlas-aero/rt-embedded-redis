use crate::network::client::CommandErrors;
use crate::network::handler::ConnectionError::{
    AuthenticationError, ProtocolSwitchError, TcpConnectionFailed, TcpSocketError,
};
use crate::network::handler::{ConnectionHandler, Credentials};
use crate::network::tests::mocks::{NetworkMockBuilder, TestClock};
use alloc::string::ToString;
use alloc::vec;
use core::str::FromStr;
use embedded_nal::SocketAddr;
use embedded_time::duration::Extensions;

#[test]
fn test_connect_new_socket_fails() {
    let clock = TestClock::new(vec![]);

    let mut stack = NetworkMockBuilder::new().socket_error().into_mock();

    let mut handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
    let result = handler.connect(&mut stack, Some(&clock));

    assert_eq!(TcpSocketError, result.unwrap_err());
}

#[test]
fn test_connect_new_connection_fail() {
    let clock = TestClock::new(vec![]);

    let mut stack = NetworkMockBuilder::new().socket(167).connect_error(167).close(167).into_mock();

    let mut handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
    let result = handler.connect(&mut stack, Some(&clock));

    assert_eq!(TcpConnectionFailed, result.unwrap_err());
}

#[test]
fn test_resp2_connect_auth_failed() {
    let clock = TestClock::new(vec![]);

    let mut stack = NetworkMockBuilder::new()
        .socket(167)
        .connect(167)
        .send(167, "")
        .response_error()
        .into_mock();

    let mut handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
    handler.auth(Credentials::password_only("secret"));
    let result = handler.connect(&mut stack, Some(&clock));

    assert_eq!(
        AuthenticationError(CommandErrors::ErrorResponse("Error".to_string())),
        result.unwrap_err()
    );
}

#[test]
fn test_resp3_connect_auth_failed() {
    let clock = TestClock::new(vec![]);

    let mut stack = NetworkMockBuilder::new()
        .socket(167)
        .connect(167)
        .send(167, "")
        .response_error()
        .into_mock();

    let mut handler = ConnectionHandler::resp3(SocketAddr::from_str("127.0.0.1:6379").unwrap());
    handler.auth(Credentials::password_only("secret"));
    let result = handler.connect(&mut stack, Some(&clock));

    assert_eq!(
        AuthenticationError(CommandErrors::ErrorResponse("Error".to_string())),
        result.unwrap_err()
    );
}

#[test]
fn test_resp3_connect_hello_failed() {
    let clock = TestClock::new(vec![]);

    let mut stack = NetworkMockBuilder::new()
        .socket(167)
        .connect(167)
        .send(167, "")
        .response_ok()
        .send_hello(167)
        .response_error()
        .into_mock();

    let mut handler = ConnectionHandler::resp3(SocketAddr::from_str("127.0.0.1:6379").unwrap());
    handler.auth(Credentials::password_only("secret"));
    let result = handler.connect(&mut stack, Some(&clock));

    assert_eq!(
        ProtocolSwitchError(CommandErrors::ErrorResponse("Error".to_string())),
        result.unwrap_err()
    );
}

#[test]
fn test_resp3_connect_hello_response() {
    let clock = TestClock::new(vec![]);

    let mut stack = NetworkMockBuilder::new()
        .socket(167)
        .connect(167)
        .send(167, "")
        .response_ok()
        .send_hello(167)
        .response_hello()
        .into_mock();

    let mut handler = ConnectionHandler::resp3(SocketAddr::from_str("127.0.0.1:6379").unwrap());
    handler.auth(Credentials::password_only("secret"));
    let result = handler.connect(&mut stack, Some(&clock)).unwrap();

    assert_eq!("redis", result.get_hello_response().server);
    assert_eq!("6.0.0", result.get_hello_response().version);
    assert_eq!(3, result.get_hello_response().protocol);
    assert_eq!(10, result.get_hello_response().id);
    assert_eq!("standalone", result.get_hello_response().mode);
    assert_eq!("master", result.get_hello_response().role);
    assert!(result.get_hello_response().modules.is_empty());
}

#[test]
fn test_resp2_connect_auth_failed_socket_closed() {
    let clock = TestClock::new(vec![]);

    let mut stack = NetworkMockBuilder::new()
        .socket(167)
        .connect(167)
        .send_error()
        .close(167)
        .socket(210)
        .connect(210)
        .send(210, "")
        .response_ok()
        .into_mock();

    let mut handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
    handler.auth(Credentials::password_only("secret"));

    // Authentication fails, so socket is expected to be closed on next connect try
    handler.connect(&mut stack, Some(&clock)).unwrap_err();
    handler.connect(&mut stack, Some(&clock)).unwrap();
}

#[test]
fn test_resp3_connect_auth_failed_socket_closed() {
    let clock = TestClock::new(vec![]);

    let mut stack = NetworkMockBuilder::new()
        .socket(167)
        .connect(167)
        .send_error()
        .close(167)
        .socket(210)
        .connect(210)
        .send(210, "")
        .response_ok()
        .send_hello(210)
        .response_hello()
        .into_mock();

    let mut handler = ConnectionHandler::resp3(SocketAddr::from_str("127.0.0.1:6379").unwrap());
    handler.auth(Credentials::password_only("secret"));

    // Authentication fails, so socket is expected to be closed on next connect try
    handler.connect(&mut stack, Some(&clock)).unwrap_err();
    handler.connect(&mut stack, Some(&clock)).unwrap();
}

#[test]
fn test_connect_resp2_socket_reused() {
    let clock = TestClock::new(vec![]);

    let mut stack = NetworkMockBuilder::new()
        .socket(167)
        .connect(167)
        .expect_is_connected(167, true)
        .into_mock();

    let mut handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());

    // Authentication fails, so socket is expected to be closed on next connect try
    handler.connect(&mut stack, Some(&clock)).unwrap();
    handler.connect(&mut stack, Some(&clock)).unwrap();
}

#[test]
fn test_connect_resp3_socket_reused() {
    let clock = TestClock::new(vec![]);

    let mut stack = NetworkMockBuilder::new()
        .socket(167)
        .connect(167)
        .send(167, "") // Auth
        .response_ok() // Auth response
        .send_hello(167)
        .response_hello()
        .expect_is_connected(167, true)
        .into_mock();

    let mut handler = ConnectionHandler::resp3(SocketAddr::from_str("127.0.0.1:6379").unwrap());
    handler.auth(Credentials::acl("test", "secret"));

    // Authentication fails, so socket is expected to be closed on next connect try
    handler.connect(&mut stack, Some(&clock)).unwrap();
    let client = handler.connect(&mut stack, Some(&clock)).unwrap();

    assert_eq!("redis", client.get_hello_response().server);
    assert_eq!("6.0.0", client.get_hello_response().version);
    assert_eq!(3, client.get_hello_response().protocol);
    assert_eq!(10, client.get_hello_response().id);
    assert_eq!("standalone", client.get_hello_response().mode);
    assert_eq!("master", client.get_hello_response().role);
    assert!(client.get_hello_response().modules.is_empty());
}

#[test]
fn test_connect_socket_is_connected_error() {
    let clock = TestClock::new(vec![]);

    let mut stack = NetworkMockBuilder::new()
        .socket(167)
        .connect(167)
        .expect_is_connected_error(167)
        .close(167)
        .socket(297)
        .connect(297)
        .into_mock();

    let mut handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());

    handler.connect(&mut stack, Some(&clock)).unwrap();
    handler.connect(&mut stack, Some(&clock)).unwrap();
}

#[test]
fn test_connect_socket_ping_tcp_error() {
    let clock = TestClock::new(vec![]);

    let mut stack = NetworkMockBuilder::new()
        .socket(167)
        .connect(167)
        .expect_is_connected(167, true)
        .send_error()
        .close(167)
        .socket(297)
        .connect(297)
        .into_mock();

    let mut handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
    handler.use_ping();

    handler.connect(&mut stack, Some(&clock)).unwrap();
    handler.connect(&mut stack, Some(&clock)).unwrap();
}

#[test]
fn test_connect_socket_ping_tcp_error_response() {
    let clock = TestClock::new(vec![]);

    let mut stack = NetworkMockBuilder::new()
        .socket(167)
        .connect(167)
        .expect_is_connected(167, true)
        .send(167, "*1\r\n$4\r\nPING\r\n")
        .response_error()
        .close(167)
        .socket(297)
        .connect(297)
        .into_mock();

    let mut handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
    handler.use_ping();

    handler.connect(&mut stack, Some(&clock)).unwrap();
    handler.connect(&mut stack, Some(&clock)).unwrap();
}

#[test]
fn test_connect_socket_ping_timeout() {
    let clock = TestClock::new(vec![
        100, // Timer creation
        200, // First receive() call
        300, // Second receive() call
    ]);

    let mut stack = NetworkMockBuilder::new()
        .socket(167)
        .connect(167)
        .expect_is_connected(167, true)
        .send(167, "*1\r\n$4\r\nPING\r\n")
        .response_no_data()
        .response_no_data()
        .close(167)
        .socket(297)
        .connect(297)
        .into_mock();

    let mut handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
    handler.timeout(150.microseconds());
    handler.use_ping();

    handler.connect(&mut stack, Some(&clock)).unwrap();
    handler.connect(&mut stack, Some(&clock)).unwrap();
}

#[test]
fn test_connect_socket_ping_successful() {
    let clock = TestClock::new(vec![]);

    let mut stack = NetworkMockBuilder::new()
        .socket(167)
        .connect(167)
        .expect_is_connected(167, true)
        .send(167, "*1\r\n$4\r\nPING\r\n")
        .response_string("PONG")
        .into_mock();

    let mut handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());
    handler.use_ping();

    handler.connect(&mut stack, Some(&clock)).unwrap();
    handler.connect(&mut stack, Some(&clock)).unwrap();
}

#[test]
fn test_connect_cached_socket_not_connected() {
    let clock = TestClock::new(vec![]);

    let mut stack = NetworkMockBuilder::new()
        .socket(167)
        .connect(167)
        .expect_is_connected(167, false)
        .close(167)
        .socket(297)
        .connect(297)
        .into_mock();

    let mut handler = ConnectionHandler::resp2(SocketAddr::from_str("127.0.0.1:6379").unwrap());

    handler.connect(&mut stack, Some(&clock)).unwrap();
    handler.connect(&mut stack, Some(&clock)).unwrap();
}
