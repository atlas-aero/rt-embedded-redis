use crate::network::protocol::Resp2;
use crate::network::response::ResponseBuffer;

#[test]
fn test_complete_empty_buffer() {
    let buffer = ResponseBuffer::new(Resp2 {});

    assert!(!buffer.is_complete(0));
}

#[test]
fn test_complete_incomplete_simple_string() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+test");

    assert!(!buffer.is_complete(0));
}

#[test]
fn test_complete_incomplete_crlf() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+test\r");

    assert!(!buffer.is_complete(0));
}

#[test]
fn test_complete_fault_prefix() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"_test\r\n");

    assert!(!buffer.is_complete(0));
}

#[test]
fn test_complete_simple_string() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+test\r\n");

    assert!(buffer.is_complete(0));
}

#[test]
fn test_complete_error_string() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"-Error\r\n");

    assert!(buffer.is_complete(0));
}

#[test]
fn test_complete_double_frame() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+Ok\r\n+Ok\r\n");

    assert!(buffer.is_complete(0));
    assert!(buffer.is_complete(1));
}

#[test]
fn test_complete_double_frame_incomplete() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+Ok\r\n+Ok\r\n+Ok\r");

    assert!(buffer.is_complete(0));
    assert!(buffer.is_complete(1));
    assert!(!buffer.is_complete(2));
}

#[test]
fn test_complete_multiple_frames_first_frame_true() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+first\r\n");
    buffer.append(b"+second\r\n");

    assert!(buffer.is_complete(0));
}

#[test]
fn test_complete_multiple_frames_second_frame_true() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+first\r\n");
    buffer.append(b"+second\r\n");

    assert!(buffer.is_complete(1));
}

#[test]
fn test_complete_multiple_frames_false() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+first\r\n");
    buffer.append(b"+second\r\n");

    assert!(!buffer.is_complete(2));
}

#[test]
fn test_complete_multiple_frames_unprocessed_data() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+first\r\n");
    buffer.append(b"+second\r\n*2");

    assert!(buffer.is_complete(0));
    assert!(buffer.is_complete(1));
}

#[test]
fn test_is_ok_response_unprocessed_data_left() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n*2");

    let frame = buffer.take_frame(0).unwrap();
    assert!(frame.is_string());
    assert_eq!("OK", frame.to_string().unwrap());
}

#[test]
fn test_is_ok_response_true() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n");

    let frame = buffer.take_frame(0).unwrap();
    assert!(frame.is_string());
    assert_eq!("OK", frame.to_string().unwrap());
}

#[test]
fn test_is_ok_response_multiple_frames_true() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"-ERROR\r\n");
    buffer.append(b"+OK\r\n");

    let frame = buffer.take_frame(1).unwrap();
    assert!(frame.is_string());
    assert_eq!("OK", frame.to_string().unwrap());
}

#[test]
fn test_is_ok_response_multiple_frames_false() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n");
    buffer.append(b"-ERROR\r\n");

    let frame = buffer.take_frame(1).unwrap();
    assert!(frame.is_error());
}

#[test]
fn test_take_frame_non_existent() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    assert!(buffer.take_frame(0).is_none())
}

#[test]
fn test_take_frame_double_call() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n");

    let first = buffer.take_frame(0);
    let second = buffer.take_frame(0);

    assert!(first.is_some());
    assert!(second.is_none());
}

#[test]
fn test_take_frame_all_frames_taken() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n");
    buffer.append(b"+OK\r\n");
    buffer.append(b"+OK\r\n");

    assert_eq!(3, buffer.pending_frame_count());
    assert!(buffer.take_frame(0).is_some());
    assert!(buffer.take_frame(1).is_some());
    assert!(buffer.take_frame(2).is_some());
    assert_eq!(0, buffer.pending_frame_count());
    assert_eq!(3, buffer.frame_offset());

    buffer.append(b"+OK\r\n");
    buffer.append(b"+OK\r\n");

    // Next assertions assert that offset is correctly applied
    assert!(buffer.is_complete(3));
    assert!(buffer.take_frame(3).is_some());

    assert!(buffer.is_complete(4));
    assert!(buffer.take_frame(4).is_some());
    assert_eq!(5, buffer.frame_offset());
}

#[test]
fn test_complete_invalid_index() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n");

    // As this is the only frame remaining, internal frame vector gets cleared and offset
    // incremented
    assert!(buffer.take_frame(0).is_some());
    assert!(!buffer.is_complete(0));
}

#[test]
fn test_take_frame_invalid_index() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n");
    buffer.append(b"+OK\r\n");

    assert!(buffer.take_frame(0).is_some());

    // As this is the only frame remaining, internal frame vector gets cleared and offset
    // incremented. So without proper index check, index underflow would occur
    assert!(buffer.take_frame(1).is_some());
    assert!(buffer.take_frame(0).is_none());
}

#[test]
fn test_faulty_unknown_frame_prefix() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n_test\r\n");

    assert!(buffer.is_faulty());
}

#[test]
fn test_faulty_false() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n");

    assert!(!buffer.is_faulty());
}

#[test]
fn test_faulty_previous_frame_readable() {
    let mut buffer = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n_test\r\n");

    assert!(buffer.is_complete(0));

    let frame = buffer.take_frame(0).unwrap();
    assert!(frame.is_string());
    assert_eq!("OK", frame.to_string().unwrap());
}
