use crate::network::protocol::Resp2;
use crate::network::response::{ResponseBuffer, ResponseBufferErrors};

type BufferType = ResponseBuffer<Resp2, 8>;

#[test]
fn test_complete_empty_buffer() {
    let buffer: BufferType = ResponseBuffer::new(Resp2 {});

    assert!(!buffer.is_complete(0));
}

#[test]
fn test_complete_incomplete_simple_string() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+test").unwrap();

    assert!(!buffer.is_complete(0));
}

#[test]
fn test_complete_incomplete_crlf() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+test\r").unwrap();

    assert!(!buffer.is_complete(0));
}

#[test]
fn test_complete_fault_prefix() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"_test\r\n").unwrap();

    assert!(!buffer.is_complete(0));
}

#[test]
fn test_complete_simple_string() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+test\r\n").unwrap();

    assert!(buffer.is_complete(0));
}

#[test]
fn test_complete_error_string() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"-Error\r\n").unwrap();

    assert!(buffer.is_complete(0));
}

#[test]
fn test_complete_double_frame() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+Ok\r\n+Ok\r\n").unwrap();

    assert!(buffer.is_complete(0));
    assert!(buffer.is_complete(1));
}

#[test]
fn test_complete_double_frame_incomplete() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+Ok\r\n+Ok\r\n+Ok\r").unwrap();

    assert!(buffer.is_complete(0));
    assert!(buffer.is_complete(1));
    assert!(!buffer.is_complete(2));
}

#[test]
fn test_complete_multiple_frames_first_frame_true() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+first\r\n").unwrap();
    buffer.append(b"+second\r\n").unwrap();

    assert!(buffer.is_complete(0));
}

#[test]
fn test_complete_multiple_frames_second_frame_true() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+first\r\n").unwrap();
    buffer.append(b"+second\r\n").unwrap();

    assert!(buffer.is_complete(1));
}

#[test]
fn test_complete_multiple_frames_false() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+first\r\n").unwrap();
    buffer.append(b"+second\r\n").unwrap();

    assert!(!buffer.is_complete(2));
}

#[test]
fn test_complete_multiple_frames_unprocessed_data() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+first\r\n").unwrap();
    buffer.append(b"+second\r\n*2").unwrap();

    assert!(buffer.is_complete(0));
    assert!(buffer.is_complete(1));
}

#[test]
fn test_is_ok_response_unprocessed_data_left() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n*2").unwrap();

    let frame = buffer.take_frame(0).unwrap();
    assert!(frame.is_string());
    assert_eq!("OK", frame.to_string().unwrap());
}

#[test]
fn test_is_ok_response_true() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n").unwrap();

    let frame = buffer.take_frame(0).unwrap();
    assert!(frame.is_string());
    assert_eq!("OK", frame.to_string().unwrap());
}

#[test]
fn test_is_ok_response_multiple_frames_true() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"-ERROR\r\n").unwrap();
    buffer.append(b"+OK\r\n").unwrap();

    let frame = buffer.take_frame(1).unwrap();
    assert!(frame.is_string());
    assert_eq!("OK", frame.to_string().unwrap());
}

#[test]
fn test_is_ok_response_multiple_frames_false() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n").unwrap();
    buffer.append(b"-ERROR\r\n").unwrap();

    let frame = buffer.take_frame(1).unwrap();
    assert!(frame.is_error());
}

#[test]
fn test_take_frame_non_existent() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    assert!(buffer.take_frame(0).is_none())
}

#[test]
fn test_take_frame_double_call() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n").unwrap();

    let first = buffer.take_frame(0);
    let second = buffer.take_frame(0);

    assert!(first.is_some());
    assert!(second.is_none());
}

#[test]
fn test_take_frame_all_frames_taken() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n").unwrap();
    buffer.append(b"+OK\r\n").unwrap();
    buffer.append(b"+OK\r\n").unwrap();

    assert_eq!(3, buffer.pending_frame_count());
    assert!(buffer.take_frame(0).is_some());
    assert!(buffer.take_frame(1).is_some());
    assert!(buffer.take_frame(2).is_some());
    assert_eq!(0, buffer.pending_frame_count());
    assert_eq!(3, buffer.frame_offset());

    buffer.append(b"+OK\r\n").unwrap();
    buffer.append(b"+OK\r\n").unwrap();

    // Next assertions assert that offset is correctly applied
    assert!(buffer.is_complete(3));
    assert!(buffer.take_frame(3).is_some());

    assert!(buffer.is_complete(4));
    assert!(buffer.take_frame(4).is_some());
    assert_eq!(5, buffer.frame_offset());
}

#[test]
fn test_complete_invalid_index() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n").unwrap();

    // As this is the only frame remaining, internal frame vector gets cleared and offset
    // incremented
    assert!(buffer.take_frame(0).is_some());
    assert!(!buffer.is_complete(0));
}

#[test]
fn test_take_frame_invalid_index() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n").unwrap();
    buffer.append(b"+OK\r\n").unwrap();

    assert!(buffer.take_frame(0).is_some());

    // As this is the only frame remaining, internal frame vector gets cleared and offset
    // incremented. So without proper index check, index underflow would occur
    assert!(buffer.take_frame(1).is_some());
    assert!(buffer.take_frame(0).is_none());
}

#[test]
fn test_faulty_unknown_frame_prefix() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n_test\r\n").unwrap();

    assert!(buffer.is_faulty());
}

#[test]
fn test_faulty_false() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n").unwrap();

    assert!(!buffer.is_faulty());
}

#[test]
fn test_faulty_previous_frame_readable() {
    let mut buffer: BufferType = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+OK\r\n_test\r\n").unwrap();

    assert!(buffer.is_complete(0));

    let frame = buffer.take_frame(0).unwrap();
    assert!(frame.is_string());
    assert_eq!("OK", frame.to_string().unwrap());
}

#[test]
fn test_append_future_overflow() {
    let mut buffer: ResponseBuffer<Resp2, 2> = ResponseBuffer::new(Resp2 {});
    buffer.append(b"+Ok\r\n").unwrap();
    buffer.append(b"+Ok\r\n").unwrap();

    let error = buffer.append(b"+Ok\r\n").unwrap_err();
    assert!(matches!(error, ResponseBufferErrors::FutureOverflow));

    buffer.take_frame(0);
    buffer.take_frame(1);

    // Asserts that buffer is cleared when all frames are taken
    buffer.append(b"+Ok\r\n").unwrap();
}
