use crate::commands::set::SetCommand;
use crate::network::buffer::Network;
use crate::network::client::Client;
use crate::network::client::CommandErrors::{
    CommandResponseViolation, ErrorResponse, InvalidFuture, ProtocolViolation, TcpError, Timeout, TimerError,
};
use crate::network::handler::ConnectionError::{AuthenticationError, ProtocolSwitchError};
use crate::network::handler::Credentials;
use crate::network::protocol::{Resp2, Resp3};
use crate::network::response::MemoryParameters;
use crate::network::tests::mocks::MockTcpError::Error1;
use crate::network::tests::mocks::{
    create_mocked_client, MockNetworkStack, NetworkMockBuilder, SocketMock, TestClock,
};
use crate::network::CommandErrors;
use alloc::string::ToString;
use alloc::vec;
use bytes::Bytes;
use embedded_time::duration::Extensions;
use futures_util::future::join;

#[async_std::test]
async fn test_resp2_init_no_authentication() {
    // By default no call to any method is expected
    let network = MockNetworkStack::new();
    let clock = TestClock::new(vec![]);
    let socket = SocketMock::new(1);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    client.init(None).await.unwrap();
}

#[async_std::test]
async fn test_resp2_init_send_tcp_error() {
    let clock = TestClock::new(vec![]);
    let mut network = MockNetworkStack::new();

    network.expect_send().times(1).returning(move |_, _| Err(Error1));

    let socket = SocketMock::new(1);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let result = client.init(Some(Credentials::password_only("test"))).await;
    assert_eq!(AuthenticationError(TcpError), result.unwrap_err());
}

#[async_std::test]
async fn test_resp2_init_correct_message_sent() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*2\r\n$4\r\nAUTH\r\n$9\r\nsecret123\r\n")
        .response_ok()
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    client.init(Some(Credentials::password_only("secret123"))).await.unwrap();
}

#[async_std::test]
async fn test_resp2_init_receive_tcp_error() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default().send(1, "").receive_tcp_error().into_mock();

    let socket = SocketMock::new(1);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let result = client.init(Some(Credentials::password_only("secret123"))).await;
    assert_eq!(AuthenticationError(TcpError), result.unwrap_err());
}

#[async_std::test]
async fn test_resp2_init_negative_response() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default().send(1, "").response_error().into_mock();

    let socket = SocketMock::new(1);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let result = client.init(Some(Credentials::password_only("secret123"))).await;
    assert_eq!(
        AuthenticationError(ErrorResponse("Error".to_string())),
        result.unwrap_err()
    );
}

#[async_std::test]
async fn test_resp2_init_response_split() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "")
        .response("+O")
        .response_no_data()
        .response("K\r\n")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    client.init(Some(Credentials::password_only("secret123"))).await.unwrap();
}

#[async_std::test]
async fn test_resp3_init_not_auth_just_hello() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default().send_hello(164).response_hello().into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp3 {});

    client.init(None).await.unwrap();
}

#[async_std::test]
async fn test_resp3_init_auth_password_only() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*2\r\n$4\r\nAUTH\r\n$9\r\nsecret123\r\n")
        .send_hello(164)
        .response_ok()
        .response_hello()
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp3 {});

    client.init(Some(Credentials::password_only("secret123"))).await.unwrap();
}

#[async_std::test]
async fn test_resp3_init_auth_acl() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*3\r\n$4\r\nAUTH\r\n$6\r\nuser01\r\n$9\r\nsecret123\r\n")
        .send_hello(164)
        .response_ok()
        .response_hello()
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp3 {});

    client.init(Some(Credentials::acl("user01", "secret123"))).await.unwrap();
}

#[async_std::test]
async fn test_resp3_init_auth_failure() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default().send(164, "").response_error().into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp3 {});

    let result = client.init(Some(Credentials::acl("user01", "secret123"))).await;
    assert_eq!(
        AuthenticationError(ErrorResponse("Error".to_string())),
        result.unwrap_err()
    )
}

#[async_std::test]
async fn test_resp3_init_hello_tcp_tx_error() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default().send_error().into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp3 {});

    let result = client.init(None).await;
    assert_eq!(ProtocolSwitchError(TcpError), result.unwrap_err())
}

#[async_std::test]
async fn test_resp3_init_hello_tcp_rx_error() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default().send(164, "").receive_tcp_error().into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp3 {});

    let result = client.init(None).await;
    assert_eq!(ProtocolSwitchError(TcpError), result.unwrap_err())
}

#[async_std::test]
async fn test_resp3_init_hello_error_response() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default().send(164, "").response_error().into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp3 {});

    let result = client.init(None).await;
    assert_eq!(
        ProtocolSwitchError(ErrorResponse("Error".to_string())),
        result.unwrap_err()
    )
}

#[async_std::test]
async fn test_timeout_expired() {
    let clock = TestClock::new(vec![
        100, // Timer creation
        200, // First receive() call
        300, // Second receive() call
    ]);

    let network = NetworkMockBuilder::default()
        .send(164, "")
        .response_no_data()
        .response_no_data()
        .response_no_data()
        .into_mock();

    let socket = SocketMock::new(164);
    let client = Client {
        network: Network::new(network.connection(socket), Resp2 {}, MemoryParameters::default()),
        timeout_duration: 150.microseconds(),
        clock: Some(&clock),
        hello_response: None,
    };

    let result = client.init(Some(Credentials::password_only("secret123"))).await;
    assert_eq!(AuthenticationError(Timeout), result.unwrap_err())
}

#[async_std::test]
async fn test_timeout_timer_error() {
    let clock = TestClock::new(vec![
        100, // Timer creation
        200, // First receive() call
    ]);

    let network = NetworkMockBuilder::default()
        .send(164, "")
        .response_no_data()
        .response_no_data()
        .into_mock();

    let socket = SocketMock::new(164);
    let client = Client {
        network: Network::new(network.connection(socket), Resp2 {}, MemoryParameters::default()),
        timeout_duration: 150.microseconds(),
        clock: Some(&clock),
        hello_response: None,
    };

    let result = client.init(Some(Credentials::password_only("secret123"))).await;
    assert_eq!(AuthenticationError(TimerError), result.unwrap_err())
}

#[async_std::test]
async fn test_timeout_not_expired() {
    let clock = TestClock::new(vec![
        100, // Timer creation
        200, // First receive() call
        300, // Second receive() call
    ]);

    let network = NetworkMockBuilder::default()
        .send(164, "")
        .response_no_data()
        .response_no_data()
        .response_ok()
        .into_mock();

    let socket = SocketMock::new(164);
    let client = Client {
        network: Network::new(network.connection(socket), Resp2 {}, MemoryParameters::default()),
        timeout_duration: 250.microseconds(),
        clock: Some(&clock),
        hello_response: None,
    };

    client.init(Some(Credentials::password_only("secret123"))).await.unwrap();
}

#[async_std::test]
async fn test_set_ok_response() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*3\r\n$3\r\nSET\r\n$8\r\ntest_key\r\n$4\r\ntest\r\n")
        .response_no_data()
        .response_ok()
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    client
        .send(SetCommand::new("test_key", "test"))
        .await
        .unwrap()
        .wait()
        .await
        .unwrap();
}

#[async_std::test]
async fn test_set_error_response() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default().send(164, "").response_error().into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let result = client
        .send(SetCommand::new("test_key", "test"))
        .await
        .unwrap()
        .wait()
        .await
        .unwrap_err();
    assert_eq!(ErrorResponse("Error".to_string()), result);
}

#[async_std::test]
async fn test_set_unknown_response() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default().send(164, "").response("+UNKNOWN\r\n").into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let response = client.send(SetCommand::new("test_key", "test")).await.unwrap().wait().await;
    assert_eq!(CommandResponseViolation, response.unwrap_err());
}

#[async_std::test]
async fn test_faulty_response() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "")
        .response("UNDEFINED\r\n")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let result = client.send(SetCommand::new("test_key", "test")).await.unwrap().wait().await;
    assert_eq!(ProtocolViolation, result.unwrap_err())
}

#[async_std::test]
async fn test_future_ready_true() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default().send(164, "").response_ok().into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let mut future = client.send(SetCommand::new("first", "future")).await.unwrap();

    assert!(future.ready().await);
    future.wait().await.unwrap();
}

#[async_std::test]
async fn test_future_ready_after_pending_read() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "")
        .response_no_data()
        .response_ok()
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let mut future = client.send(SetCommand::new("first", "future")).await.unwrap();
    assert!(future.ready().await);
    future.wait().await.unwrap();
}

#[async_std::test]
async fn test_future_ready_after_incomplete_frame() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "")
        .response("+O")
        .response_no_data()
        .response("K\r\n")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let mut future = client.send(SetCommand::new("first", "future")).await.unwrap();

    assert!(future.ready().await);
    future.wait().await.unwrap();
}

#[async_std::test]
async fn test_future_ready_error() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default().send(164, "").receive_tcp_error().into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let mut future = client.send(SetCommand::new("first", "future")).await.unwrap();

    assert!(future.ready().await);
    assert_eq!(TcpError, future.wait().await.unwrap_err());
}

#[async_std::test]
async fn test_future_reports_closed_connection() {
    let clock = TestClock::new(vec![]);
    let network = NetworkMockBuilder::default().send(164, "").response_eof().into_mock();
    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let future = client.send(SetCommand::new("first", "future")).await.unwrap();

    assert_eq!(TcpError, future.wait().await.unwrap_err());
}

#[async_std::test]
/// Tests asserts if futures are called in sequence
async fn test_multiple_responses_future_wait_in_order() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "")
        .send(164, "")
        .response_error()
        .response("+O")
        .response_no_data()
        .response("K\r\n")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let first = client.send(SetCommand::new("first", "future")).await.unwrap();
    let second = client.send(SetCommand::new("second", "future")).await.unwrap();

    assert_eq!(
        ErrorResponse("Error".to_string()),
        first.wait().await.unwrap_err()
    );
    second.wait().await.unwrap();
}

#[async_std::test]
/// Tests asserts if futures are called out of order
async fn test_multiple_responses_future_wait_crossed() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "")
        .send(164, "")
        .response_error()
        .response("+O")
        .response_no_data()
        .response("K\r\n")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let first = client.send(SetCommand::new("first", "future")).await.unwrap();
    let second = client.send(SetCommand::new("second", "future")).await.unwrap();

    second.wait().await.unwrap();
    assert_eq!(
        ErrorResponse("Error".to_string()),
        first.wait().await.unwrap_err()
    );
}

#[async_std::test]
async fn test_multiple_responses_can_be_awaited_concurrently() {
    let clock = TestClock::new(vec![]);
    let network = NetworkMockBuilder::default()
        .send(164, "")
        .send(164, "")
        .response("+OK\r\n+OK\r\n")
        .into_mock();
    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let first = client.send(SetCommand::new("first", "future")).await.unwrap();
    let second = client.send(SetCommand::new("second", "future")).await.unwrap();
    let (first_result, second_result) = join(first.wait(), second.wait()).await;

    first_result.unwrap();
    second_result.unwrap();
}

#[async_std::test]
async fn test_multiple_responses_partly_complete() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "")
        .send(164, "")
        .response_ok()
        .response("+O")
        .response_no_data()
        .response("K\r\n")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let mut first = client.send(SetCommand::new("first", "future")).await.unwrap();
    let mut second = client.send(SetCommand::new("second", "future")).await.unwrap();

    assert!(first.ready().await);
    assert!(second.ready().await);
    first.wait().await.unwrap();
    second.wait().await.unwrap();
}

#[async_std::test]
async fn test_futures_invalidated_on_timeout() {
    let clock = TestClock::new(vec![
        100, // Timer creation
        101, // Timer creation
        200, // First receive() call
        300, // Second receive() call
    ]);

    let network = NetworkMockBuilder::default()
        .send(164, "")
        .send(164, "")
        .response_no_data()
        .response_no_data()
        .into_mock();

    let socket = SocketMock::new(164);
    let client = Client {
        network: Network::new(network.connection(socket), Resp2 {}, MemoryParameters::default()),
        timeout_duration: 150.microseconds(),
        clock: Some(&clock),
        hello_response: None,
    };

    let first = client.send(SetCommand::new("timeout", "future")).await.unwrap();
    let second = client.send(SetCommand::new("second", "future")).await.unwrap();
    assert_eq!(Timeout, first.wait().await.unwrap_err());
    assert_eq!(InvalidFuture, second.wait().await.unwrap_err());
}

#[async_std::test]
async fn test_future_invalidated_on_faulty_response() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "")
        .send(164, "")
        .response("_faulty\r\n")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let first = client.send(SetCommand::new("faulty", "future")).await.unwrap();
    let second = client.send(SetCommand::new("second", "future")).await.unwrap();

    assert_eq!(ProtocolViolation, first.wait().await.unwrap_err());
    assert_eq!(InvalidFuture, second.wait().await.unwrap_err());

    // A protocol violation is fatal; the async connection must be dropped instead of drained.
}

/// Tests dropped future, which wait() method was not called.
/// Response data of this futures is handled at next send() call
/// In the following scenario the data arrives at the next send call
#[async_std::test]
async fn test_future_dropped_received_at_send() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "")
        .send(164, "")
        .response_ok()
        .response_no_data()
        .response_ok()
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    {
        let _ = client.send(SetCommand::new("key", "value")).await;
    }

    assert_eq!(1, client.network.get_dropped_future_count());
    let future = client.send(SetCommand::new("key", "value")).await.unwrap();
    assert_eq!(1, client.network.get_dropped_future_count());
    assert_eq!(0, client.network.get_pending_frame_count());
    future.wait().await.unwrap();
    assert_eq!(1, client.network.get_pending_frame_count());
}

/// Tests dropped future, which wait() method was not called.
/// Response data of this futures is handled at next send() call
/// In the following scenario the data arrives at the next future wait() call
#[async_std::test]
async fn test_future_dropped_received_at_next_future() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "")
        .send(164, "")
        .send(164, "")
        .response_no_data() // Called at second send, no data arrived yet
        .response_ok() // Data of first (dropped) future
        .response_ok() // Data of second future, which wait() method is called
        .response_no_data() // Called a third send, no more data to receive
        .response_ok() // Data of third future
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    {
        let _ = client.send(SetCommand::new("key", "value")).await;
    }

    assert_eq!(1, client.network.get_dropped_future_count());
    let second = client.send(SetCommand::new("key", "value")).await.unwrap();
    // Data of dropped future is not arrived yet
    assert_eq!(1, client.network.get_dropped_future_count());
    assert_eq!(0, client.network.get_pending_frame_count());

    // Data of dropped future arrives now
    second.wait().await.unwrap();
    assert_eq!(1, client.network.get_pending_frame_count());

    // Data of dropped future gets cleared
    assert_eq!(1, client.network.get_dropped_future_count());
    let third = client.send(SetCommand::new("key", "value")).await.unwrap();
    assert_eq!(0, client.network.get_dropped_future_count());
    assert_eq!(0, client.network.get_pending_frame_count());

    third.wait().await.unwrap();
}

/// Tests dropped future, which wait() method was not called.
/// Response data of this futures is handled at next send() call
/// In the following scenario a fatal error occurred, so the dropped future got invalidated in the
/// meanwhile
#[async_std::test]
async fn test_future_dropped_invalidated() {
    let clock = TestClock::new(vec![
        100, // Timer creation of first future
        101, // Timer creation of second future
        200, // First receive() call of first future
        300, // Second receive() call of first future <-- Timeout threshold is reached here
    ]);

    let network = NetworkMockBuilder::default()
        .send(164, "")
        .send(164, "")
        .response_no_data()
        .response_no_data()
        .into_mock();

    let socket = SocketMock::new(164);
    let client = Client {
        network: Network::new(network.connection(socket), Resp2 {}, MemoryParameters::default()),
        timeout_duration: 150.microseconds(),
        clock: Some(&clock),
        hello_response: None,
    };

    let first = client.send(SetCommand::new("timeout", "future")).await.unwrap();
    {
        let _second = client.send(SetCommand::new("second", "future")).await.unwrap();
    }
    assert_eq!(Timeout, first.wait().await.unwrap_err());

    // Second future is invalidated and can be removed without reading the connection.
    assert_eq!(1, client.network.get_dropped_future_count());
    client.network.handle_dropped_futures();
    assert_eq!(0, client.network.get_dropped_future_count());
}

#[async_std::test]
async fn test_close_timeout() {
    let clock = TestClock::new(vec![
        100, // Timer creation in future
        101, // Timer creation in close
        200, // Before first receive() call
        210, // Before second receive() call
        300, // Before third receive() call
    ]);

    let network = NetworkMockBuilder::default()
        .send(164, "")
        .response_no_data()
        .response_no_data()
        .response_no_data()
        .into_mock();

    let socket = SocketMock::new(164);
    let client = Client {
        network: Network::new(network.connection(socket), Resp2 {}, MemoryParameters::default()),
        timeout_duration: 150.microseconds(),
        clock: Some(&clock),
        hello_response: None,
    };

    {
        let _ = client.send(SetCommand::new("key", "value")).await;
    }

    assert_eq!(1, client.network.get_dropped_future_count());
    client.close().await;
    assert_eq!(1, client.network.get_dropped_future_count());
}

#[async_std::test]
async fn test_close_handled_dropped_futures() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "")
        .response_no_data()
        .response_ok()
        .response_no_data()
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    {
        let _ = client.send(SetCommand::new("key", "value")).await;
    }

    assert_eq!(1, client.network.get_dropped_future_count());
    client.close().await;
    assert_eq!(0, client.network.get_dropped_future_count());
    assert_eq!(0, client.network.get_pending_frame_count());
}

#[async_std::test]
async fn test_memory_limit_reached() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n")
        .response_incomplete_binary::<110>()
        .into_mock();

    let socket = SocketMock::new(164);
    let client = Client {
        network: Network::new(
            network.connection(socket),
            Resp3 {},
            MemoryParameters {
                buffer_size: 128,
                frame_capacity: 1,
                memory_limit: Some(100),
            },
        ),
        timeout_duration: 0.microseconds(),
        clock: Some(&clock),
        hello_response: None,
    };

    let error = client.get("key").await.unwrap().wait().await.unwrap_err();
    assert_eq!(CommandErrors::MemoryFull, error);
}

#[async_std::test]
async fn test_memory_limit_not_reached() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n")
        .response_incomplete_binary::<110>()
        .response("\r\n")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = Client {
        network: Network::new(
            network.connection(socket),
            Resp3 {},
            MemoryParameters {
                buffer_size: 128,
                frame_capacity: 1,
                memory_limit: Some(150),
            },
        ),
        timeout_duration: 0.microseconds(),
        clock: Some(&clock),
        hello_response: None,
    };

    let data = client.get("key").await.unwrap().wait().await.unwrap().unwrap().to_bytes();
    assert_eq!(&[0x0u8; 110], &data[..])
}

#[async_std::test]
async fn test_shorthand_get_str_argument() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n")
        .response_string("test_response")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    assert_eq!(
        "test_response",
        client
            .get("key")
            .await
            .unwrap()
            .wait()
            .await
            .unwrap()
            .unwrap()
            .as_str()
            .unwrap()
    );
}

#[async_std::test]
async fn test_shorthand_get_string_argument() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*2\r\n$3\r\nGET\r\n$8\r\ntest_key\r\n")
        .response_string("test_response")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let response = client.get("test_key".to_string()).await.unwrap().wait().await;
    assert_eq!("test_response", response.unwrap().unwrap().as_str().unwrap());
}

#[async_std::test]
async fn test_shorthand_get_bytes_argument() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*2\r\n$3\r\nGET\r\n$8\r\ntest_key\r\n")
        .response_string("test_response")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let response = client.get(Bytes::from_static(b"test_key")).await.unwrap().wait().await;
    assert_eq!("test_response", response.unwrap().unwrap().as_str().unwrap());
}

#[async_std::test]
async fn test_shorthand_get_multi() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(897, "*2\r\n$3\r\nGET\r\n$4\r\nkey1\r\n")
        .response_string("value1")
        .send(897, "*2\r\n$3\r\nGET\r\n$4\r\nkey2\r\n")
        .response_string("value2")
        .send(897, "*2\r\n$3\r\nGET\r\n$4\r\nkey3\r\n")
        .response_string("value3")
        .into_mock();

    let socket = SocketMock::new(897);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let response1 = client.get(Bytes::from_static(b"key1")).await.unwrap().wait().await;
    let response2 = client.get(Bytes::from_static(b"key2")).await.unwrap().wait().await;
    let response3 = client.get(Bytes::from_static(b"key3")).await.unwrap().wait().await;

    assert_eq!("value1", response1.unwrap().unwrap().as_string().unwrap());
    assert_eq!("value2", response2.unwrap().unwrap().as_string().unwrap());
    assert_eq!("value3", response3.unwrap().unwrap().as_string().unwrap());
}

#[async_std::test]
async fn test_shorthand_set_str_argument() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n")
        .response_ok()
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    client.set("key", "value").await.unwrap().wait().await.unwrap();
}

#[async_std::test]
async fn test_shorthand_set_string_argument() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n")
        .response_ok()
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    client
        .set("key".to_string(), "value".to_string())
        .await
        .unwrap()
        .wait()
        .await
        .unwrap();
}

#[async_std::test]
async fn test_shorthand_set_bytes_argument() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n")
        .response_ok()
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let key = Bytes::from_static(b"key");
    let value = Bytes::from_static(b"value");
    client.set(key, value).await.unwrap().wait().await.unwrap();
}

#[async_std::test]
async fn test_shorthand_publish() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*3\r\n$7\r\nPUBLISH\r\n$6\r\ncolors\r\n$6\r\norange\r\n")
        .response(":3\r\n")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let response = client.publish("colors", "orange").await.unwrap().wait().await.unwrap();
    assert_eq!(3, response);
}

#[async_std::test]
async fn test_shorthand_ping() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*1\r\n$4\r\nPING\r\n")
        .response_string("PONG")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    client.ping().await.unwrap().wait().await.unwrap();
}

#[async_std::test]
async fn test_shorthand_bgsave_non_scheduled() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*1\r\n$6\r\nBGSAVE\r\n")
        .response_string("Background saving started")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    client.bgsave(false).await.unwrap().wait().await.unwrap();
}

#[async_std::test]
async fn test_shorthand_bgsave_scheduled() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*2\r\n$6\r\nBGSAVE\r\n$8\r\nSCHEDULE\r\n")
        .response_string("Background saving sch")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    client.bgsave(true).await.unwrap().wait().await.unwrap();
}

#[async_std::test]
async fn test_shorthand_hset_str_argument() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(
            164,
            "*4\r\n$4\r\nHSET\r\n$7\r\nmy_hash\r\n$5\r\ncolor\r\n$5\r\ngreen\r\n",
        )
        .response(":1\r\n")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    client.hset("my_hash", "color", "green").await.unwrap().wait().await.unwrap();
}

#[async_std::test]
async fn test_shorthand_hset_string_argument() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(
            164,
            "*4\r\n$4\r\nHSET\r\n$7\r\nmy_hash\r\n$5\r\ncolor\r\n$5\r\ngreen\r\n",
        )
        .response(":1\r\n")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    client
        .hset("my_hash".to_string(), "color".to_string(), "green".to_string())
        .await
        .unwrap()
        .wait()
        .await
        .unwrap();
}

#[async_std::test]
async fn test_shorthand_hset_bytes_argument() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(
            164,
            "*4\r\n$4\r\nHSET\r\n$7\r\nmy_hash\r\n$5\r\ncolor\r\n$5\r\ngreen\r\n",
        )
        .response(":1\r\n")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    client
        .hset(
            Bytes::from_static(b"my_hash"),
            Bytes::from_static(b"color"),
            Bytes::from_static(b"green"),
        )
        .await
        .unwrap()
        .wait()
        .await
        .unwrap();
}

#[async_std::test]
async fn test_shorthand_hget_str_argument() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*3\r\n$4\r\nHGET\r\n$7\r\nmy_hash\r\n$5\r\nfield\r\n")
        .response_string("test_response")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    assert_eq!(
        "test_response",
        client
            .hget("my_hash", "field")
            .await
            .unwrap()
            .wait()
            .await
            .unwrap()
            .unwrap()
            .as_str()
            .unwrap()
    );
}

#[async_std::test]
async fn test_shorthand_hget_string_argument() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*3\r\n$4\r\nHGET\r\n$7\r\nmy_hash\r\n$5\r\nfield\r\n")
        .response_string("test_response")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let response = client
        .hget("my_hash".to_string(), "field".to_string())
        .await
        .unwrap()
        .wait()
        .await;
    assert_eq!("test_response", response.unwrap().unwrap().as_str().unwrap());
}

#[async_std::test]
async fn test_shorthand_hget_bytes_argument() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*3\r\n$4\r\nHGET\r\n$7\r\nmy_hash\r\n$5\r\nfield\r\n")
        .response_string("test_response")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    let response = client
        .hget(Bytes::from_static(b"my_hash"), Bytes::from_static(b"field"))
        .await
        .unwrap()
        .wait()
        .await;
    assert_eq!("test_response", response.unwrap().unwrap().as_str().unwrap());
}

#[async_std::test]
async fn test_shorthand_hgetall_str_argument() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*2\r\n$7\r\nHGETALL\r\n$7\r\nmy_hash\r\n")
        .response("*2\r\n$5\r\ncolor\r\n$5\r\ngreen\r\n")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    assert_eq!(
        "green",
        client
            .hgetall("my_hash")
            .await
            .unwrap()
            .wait()
            .await
            .unwrap()
            .unwrap()
            .get_str("color")
            .unwrap()
    );
}

#[async_std::test]
async fn test_shorthand_hgetall_string_argument() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*2\r\n$7\r\nHGETALL\r\n$7\r\nmy_hash\r\n")
        .response("*2\r\n$5\r\ncolor\r\n$5\r\ngreen\r\n")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    assert_eq!(
        "green",
        client
            .hgetall("my_hash".to_string())
            .await
            .unwrap()
            .wait()
            .await
            .unwrap()
            .unwrap()
            .get_str("color")
            .unwrap()
    );
}

#[async_std::test]
async fn test_shorthand_hgetall_bytes_argument() {
    let clock = TestClock::new(vec![]);

    let network = NetworkMockBuilder::default()
        .send(164, "*2\r\n$7\r\nHGETALL\r\n$7\r\nmy_hash\r\n")
        .response("*2\r\n$5\r\ncolor\r\n$5\r\ngreen\r\n")
        .into_mock();

    let socket = SocketMock::new(164);
    let client = create_mocked_client(&network, socket, &clock, Resp2 {});

    assert_eq!(
        "green",
        client
            .hgetall(Bytes::from_static(b"my_hash"))
            .await
            .unwrap()
            .wait()
            .await
            .unwrap()
            .unwrap()
            .get_str("color")
            .unwrap()
    );
}
