use crate::network::ConnectionHandler;
use bytes::Bytes;
use core::str::FromStr;
use embedded_nal::SocketAddr;
use embedded_time::duration::Microseconds;
use std_embedded_nal::Stack;
use std_embedded_time::StandardClock;
use test::Bencher;

macro_rules! setup_client {
    ($client:ident, $future_size:expr) => {
        let mut stack = Stack::default();
        let clock = StandardClock::default();

        let server_address = SocketAddr::from_str("127.0.0.1:6379").unwrap();
        let mut connection_handler = ConnectionHandler::resp3(server_address);
        connection_handler.timeout(Microseconds::new(100_000));
        let $client = connection_handler.connect::<_, $future_size>(&mut stack, Some(&clock)).unwrap();
    };
}

#[bench]
fn benchmark_publish_async(bencher: &mut Bencher) {
    setup_client!(client, 512);

    let topic = Bytes::from_static(b"test");
    let data = Bytes::from_static(&[b'A'; 256]);

    bencher.iter(|| {
        let _ = client.publish(topic.clone(), data.clone());
    });

    client.close();
}

#[bench]
fn benchmark_publish_sync(bencher: &mut Bencher) {
    setup_client!(client, 1);

    let topic = Bytes::from_static(b"test");
    let data = Bytes::from_static(&[b'A'; 256]);

    bencher.iter(|| {
        client.publish(topic.clone(), data.clone()).unwrap().wait().unwrap();
    });

    client.close();
}
