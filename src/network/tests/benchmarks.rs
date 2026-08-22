use crate::network::ConnectionHandler;
use async_std::task::block_on;
use bytes::Bytes;
use core::net::SocketAddr;
use core::str::FromStr;
use std_embedded_nal_async::Stack;
use std_embedded_time::StandardClock;
use test::Bencher;

macro_rules! setup_client {
    ($client:ident) => {
        let stack = Stack::default();
        let clock = StandardClock::default();

        let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
        let connection_handler = ConnectionHandler::resp3(server_address);
        let $client = block_on(connection_handler.connect(&stack, Some(&clock))).unwrap();
    };
}

#[bench]
fn benchmark_publish_async(bencher: &mut Bencher) {
    setup_client!(client);

    let topic = Bytes::from_static(b"test");
    let data = Bytes::from_static(&[b'A'; 256]);

    bencher.iter(|| {
        let _ = block_on(client.publish(topic.clone(), data.clone()));
    });

    block_on(client.close());
}

#[bench]
fn benchmark_publish_sync(bencher: &mut Bencher) {
    setup_client!(client);

    let topic = Bytes::from_static(b"test");
    let data = Bytes::from_static(&[b'A'; 256]);

    bencher.iter(|| {
        block_on(async {
            client.publish(topic.clone(), data.clone()).await.unwrap().wait().await.unwrap();
        });
    });

    block_on(client.close());
}
