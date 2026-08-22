use crate::network::client::CommandErrors;
use crate::network::handler::ConnectionError::{
    AuthenticationError, ProtocolSwitchError, TcpConnectionFailed,
};
use crate::network::handler::{ConnectionHandler, Credentials};
use crate::network::tests::mocks::{NetworkMockBuilder, TestClock};
use alloc::string::ToString;
use alloc::vec;
use core::net::SocketAddr;
use core::str::FromStr;

fn remote() -> SocketAddr {
    SocketAddr::from_str("127.0.0.1:6379").unwrap()
}

#[async_std::test]
async fn test_connect_fails() {
    let clock = TestClock::new(vec![]);
    let stack = NetworkMockBuilder::default().socket_error().into_mock();
    let handler = ConnectionHandler::resp2(remote());

    let result = handler.connect(&stack, Some(&clock)).await;

    assert_eq!(TcpConnectionFailed, result.unwrap_err());
}

#[async_std::test]
async fn test_resp2_connect_auth_failed() {
    let clock = TestClock::new(vec![]);
    let stack = NetworkMockBuilder::default()
        .socket(167)
        .connect(167)
        .send(167, "")
        .response_error()
        .into_mock();
    let mut handler = ConnectionHandler::resp2(remote());
    handler.auth(Credentials::password_only("secret"));

    let result = handler.connect(&stack, Some(&clock)).await;

    assert_eq!(
        AuthenticationError(CommandErrors::ErrorResponse("Error".to_string())),
        result.unwrap_err()
    );
}

#[async_std::test]
async fn test_resp3_connect_hello_failed() {
    let clock = TestClock::new(vec![]);
    let stack = NetworkMockBuilder::default()
        .socket(167)
        .connect(167)
        .send_hello(167)
        .response_error()
        .into_mock();
    let handler = ConnectionHandler::resp3(remote());

    let result = handler.connect(&stack, Some(&clock)).await;

    assert_eq!(
        ProtocolSwitchError(CommandErrors::ErrorResponse("Error".to_string())),
        result.unwrap_err()
    );
}

#[async_std::test]
async fn test_resp3_connect_exposes_hello_response() {
    let clock = TestClock::new(vec![]);
    let stack = NetworkMockBuilder::default()
        .socket(167)
        .connect(167)
        .send_hello(167)
        .response_hello()
        .into_mock();
    let handler = ConnectionHandler::resp3(remote());

    let client = handler.connect(&stack, Some(&clock)).await.unwrap();

    assert_eq!("redis", client.get_hello_response().server);
}

#[async_std::test]
async fn test_connect_can_verify_connection_with_ping() {
    let clock = TestClock::new(vec![]);
    let stack = NetworkMockBuilder::default()
        .socket(167)
        .connect(167)
        .send(167, "*1\r\n$4\r\nPING\r\n")
        .response("+PONG\r\n")
        .into_mock();
    let mut handler = ConnectionHandler::resp2(remote());
    handler.use_ping();

    handler.connect(&stack, Some(&clock)).await.unwrap();
}
