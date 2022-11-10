use crate::network::buffer::Network;
use crate::network::tests::mocks::{create_mocked_client, NetworkMockBuilder};
use crate::network::tests::mocks::{SocketMock, TestClock};
use crate::network::{Client, MemoryParameters, Resp3};
use crate::subscribe::client::Error;
use embedded_time::duration::Extensions;
use std::cell::RefCell;

#[test]
fn test_subscribe_confirmation_tcp_error() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::default()
        .send(164, "*2\r\n$9\r\nSUBSCRIBE\r\n$10\r\ntest_topic\r\n")
        .receive_tcp_error()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let error = create_mocked_client(&mut network, &mut socket, &clock, Resp3 {})
        .subscribe(["test_topic".into()])
        .unwrap_err();

    assert_eq!(Error::TcpError, error);
}

#[test]
fn test_subscribe_confirmation_single_channel() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::default()
        .send(164, "*2\r\n$9\r\nSUBSCRIBE\r\n$10\r\ntest_topic\r\n")
        .sub_confirmation_resp3("test_topic", 1)
        .response_no_data()
        .into_mock();

    let mut socket = SocketMock::new(164);
    create_mocked_client(&mut network, &mut socket, &clock, Resp3 {})
        .subscribe(["test_topic".into()])
        .unwrap();
}

#[test]
fn test_subscribe_confirmation_multi_channel() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::default()
        .send(
            164,
            "*4\r\n$9\r\nSUBSCRIBE\r\n$5\r\nfirst\r\n$6\r\nsecond\r\n$5\r\nthird\r\n",
        )
        .sub_confirmation_resp3("first", 1)
        .response_no_data()
        .sub_confirmation_resp3("second", 2)
        .response_no_data()
        .sub_confirmation_resp3("third", 3)
        .response_no_data()
        .into_mock();

    let mut socket = SocketMock::new(164);
    create_mocked_client(&mut network, &mut socket, &clock, Resp3 {})
        .subscribe(["first".into(), "second".into(), "third".into()])
        .unwrap();
}

#[test]
fn test_subscribe_confirmation_other_responses_ignored() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::default()
        .send(164, "*2\r\n$9\r\nSUBSCRIBE\r\n$10\r\ntest_topic\r\n")
        .response_string("other message")
        .response_no_data()
        .sub_confirmation_resp3("test_topic", 1)
        .response_no_data()
        .into_mock();

    let mut socket = SocketMock::new(164);
    create_mocked_client(&mut network, &mut socket, &clock, Resp3 {})
        .subscribe(["test_topic".into()])
        .unwrap();
}

#[test]
fn test_subscribe_confirmation_unknown_push_message_ignored() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::default()
        .send(164, "*2\r\n$9\r\nSUBSCRIBE\r\n$10\r\ntest_topic\r\n")
        .response(">4\r\n+status\r\n+test\r\n+t\r\n+t\r\n")
        .response_no_data()
        .sub_confirmation_resp3("test_topic", 1)
        .response_no_data()
        .into_mock();

    let mut socket = SocketMock::new(164);
    create_mocked_client(&mut network, &mut socket, &clock, Resp3 {})
        .subscribe(["test_topic".into()])
        .unwrap();
}

#[test]
fn test_subscribe_confirmation_unknown_pub_sub_type_ignored() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::default()
        .send(164, "*2\r\n$9\r\nSUBSCRIBE\r\n$10\r\ntest_topic\r\n")
        .response(">3\r\n+new_type\r\n+t\r\n+t\r\n")
        .response_no_data()
        .sub_confirmation_resp3("test_topic", 1)
        .response_no_data()
        .into_mock();

    let mut socket = SocketMock::new(164);
    create_mocked_client(&mut network, &mut socket, &clock, Resp3 {})
        .subscribe(["test_topic".into()])
        .unwrap();
}

#[test]
fn test_subscribe_confirmation_protocol_violation() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::default()
        .send(164, "*2\r\n$9\r\nSUBSCRIBE\r\n$10\r\ntest_topic\r\n")
        .response(">3\r\n+subscribe\r\n")
        .response("+channel\r\n+no_number\r\n")
        .response_no_data()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let error = create_mocked_client(&mut network, &mut socket, &clock, Resp3 {})
        .subscribe(["test_topic".into()])
        .unwrap_err();

    assert_eq!(Error::DecodeError, error);
}

#[test]
fn test_subscribe_confirmation_within_timeout() {
    let clock = TestClock::new(vec![
        1,   // Timer creation
        50,  // First receive() call
        100, // Second receive() call
    ]);

    let mut network = NetworkMockBuilder::default()
        .send(164, "")
        .sub_confirmation_resp3("test_topic", 1)
        .response_no_data()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = Client {
        network: Network::new(
            RefCell::new(&mut network),
            RefCell::new(&mut socket),
            Resp3 {},
            MemoryParameters::default(),
        ),
        timeout_duration: 150.microseconds(),
        clock: Some(&clock),
        hello_response: None,
    };

    client.subscribe(["test_topic".into()]).unwrap();
}

#[test]
fn test_subscribe_confirmation_timeout() {
    let clock = TestClock::new(vec![
        1,   // Timer creation
        50,  // First receive() call
        100, // Second receive() call
        200, // Before third receive() call
    ]);

    let mut network = NetworkMockBuilder::default()
        .send(164, "")
        .response_no_data()
        .response_no_data()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let client = Client {
        network: Network::new(
            RefCell::new(&mut network),
            RefCell::new(&mut socket),
            Resp3 {},
            MemoryParameters::default(),
        ),
        timeout_duration: 150.microseconds(),
        clock: Some(&clock),
        hello_response: None,
    };

    let error = client.subscribe(["test_topic".into()]).unwrap_err();
    assert_eq!(Error::Timeout, error);
}

#[test]
fn test_receive_other_responses_ignored() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::default()
        .send(164, "")
        .sub_confirmation_resp3("test_topic", 1)
        .response_no_data()
        .response_string("other message")
        .response_no_data()
        .response_no_data()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let mut client = create_mocked_client(&mut network, &mut socket, &clock, Resp3 {})
        .subscribe(["test_topic".into()])
        .unwrap();

    assert!(client.receive().unwrap().is_none());
}

#[test]
fn test_receive_other_unknown_push_message_ignored() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::default()
        .send(164, "")
        .sub_confirmation_resp3("test_topic", 1)
        .response_no_data()
        .response(">4\r\n+status\r\n+test\r\n+t\r\n+t\r\n")
        .response_no_data()
        .response_no_data()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let mut client = create_mocked_client(&mut network, &mut socket, &clock, Resp3 {})
        .subscribe(["test_topic".into()])
        .unwrap();

    assert!(client.receive().unwrap().is_none());
}

#[test]
fn test_receive_other_unknown_sub_type_ignored() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::default()
        .send(164, "")
        .sub_confirmation_resp3("test_topic", 1)
        .response_no_data()
        .response(">3\r\n+new_type\r\n+t\r\n+t\r\n")
        .response_no_data()
        .response_no_data()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let mut client = create_mocked_client(&mut network, &mut socket, &clock, Resp3 {})
        .subscribe(["test_topic".into()])
        .unwrap();

    assert!(client.receive().unwrap().is_none());
}

#[test]
fn test_receive_correct_message() {
    let clock = TestClock::new(vec![]);

    let mut network = NetworkMockBuilder::default()
        .send(164, "")
        .sub_confirmation_resp3("test_topic", 1)
        .response_no_data()
        .response_string("other message")
        .sub_message("test_channel", "test_payload")
        .response_no_data()
        .response_no_data()
        .into_mock();

    let mut socket = SocketMock::new(164);
    let mut client = create_mocked_client(&mut network, &mut socket, &clock, Resp3 {})
        .subscribe(["test_topic".into()])
        .unwrap();

    let message = client.receive().unwrap().unwrap();
    assert_eq!(
        "test_channel",
        core::str::from_utf8(&message.channel[..]).unwrap()
    );
    assert_eq!(
        "test_payload",
        core::str::from_utf8(&message.payload[..]).unwrap()
    );
}
