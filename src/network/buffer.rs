use crate::network::client::CommandErrors;
use crate::network::future::Identity;
use crate::network::protocol::Protocol;
use crate::network::response::{MemoryParameters, ResponseBuffer};
use alloc::vec;
use alloc::vec::Vec;
use bytes::BytesMut;
use core::cell::RefCell;
use core::fmt::{Debug, Formatter};
use core::ops::Deref;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_io_async::{Read, Write};
use redis_protocol::error::RedisProtocolErrorKind::BufferTooSmall;

/// Manages interaction between the network connection and response buffer.
pub(crate) struct Network<T: Read + Write, P: Protocol> {
    protocol: P,
    connection: Mutex<NoopRawMutex, T>,
    buffer: RefCell<ResponseBuffer<P>>,

    /// Current valid pending-response series
    current_series: RefCell<usize>,

    /// Index of the next pending response
    next_index: RefCell<usize>,

    /// Indicates a pending buffer clearance on fatal errors
    clear_buffer: RefCell<bool>,

    /// List of dropped pending responses that were not awaited
    /// For not leaking memory, response data of this futures is dropped on next send() call
    dropped_futures: RefCell<Vec<Identity>>,
}

impl<T: Read + Write, P: Protocol> Network<T, P> {
    pub(crate) fn new(connection: T, protocol: P, memory: MemoryParameters) -> Self {
        Network {
            protocol: protocol.clone(),
            connection: Mutex::new(connection),
            buffer: RefCell::new(ResponseBuffer::new(protocol, memory)),
            current_series: RefCell::new(0),
            next_index: RefCell::new(0),
            clear_buffer: RefCell::new(false),
            dropped_futures: RefCell::new(vec![]),
        }
    }

    /// Appends up to 32 bytes to the response buffer
    pub(crate) async fn receive_chunk(&self) -> Result<usize, T::Error> {
        let mut local_buffer: [u8; 32] = [0; 32];
        let mut connection = self.connection.lock().await;

        match connection.read(&mut local_buffer).await {
            Ok(byte_count) => {
                self.buffer.borrow_mut().append(&local_buffer[0..byte_count]);
                Ok(byte_count)
            }
            Err(error) => Err(error),
        }
    }

    /// Returns true if the memory limit is reached
    pub(crate) fn is_buffer_full(&self) -> bool {
        self.buffer.borrow().is_full()
    }

    /// Encodes and sends the given command
    pub(crate) async fn send(&self, frame: P::FrameType) -> Result<Identity, CommandErrors> {
        // A fatal error invalidated the current series, so everything needs to be cleared
        if *self.clear_buffer.borrow().deref() {
            self.clear_socket();
            *self.clear_buffer.borrow_mut() = false;
        }

        // Handle dropped futures for not leaking memory
        self.handle_dropped_futures();

        self.send_frame(frame).await?;

        let identity = Identity {
            series: *self.current_series.borrow(),
            index: *self.next_index.borrow(),
        };
        *self.next_index.borrow_mut() += 1;
        Ok(identity)
    }

    /// Raw network logic for sending a frame
    pub(crate) async fn send_frame(&self, frame: P::FrameType) -> Result<(), CommandErrors> {
        let mut buffer = BytesMut::new();

        // Extend buffer if needed
        while let Err(error) = self.protocol.encode_bytes(&mut buffer, &frame) {
            if let BufferTooSmall(size) = error.kind() {
                buffer.resize(buffer.len() + *size, 0x0);
            } else {
                return Err(CommandErrors::EncodingCommandFailed);
            }
        }

        let mut connection = self.connection.lock().await;

        if connection.write_all(buffer.as_ref()).await.is_err() || connection.flush().await.is_err() {
            return Err(CommandErrors::TcpError);
        };

        Ok(())
    }

    /// Is the message of the given future complete?
    pub(crate) fn is_complete(&self, id: &Identity) -> Result<bool, CommandErrors> {
        if self.current_series.borrow().deref() != &id.series {
            return Err(CommandErrors::InvalidFuture);
        }

        if self.buffer.borrow().is_complete(id.index) {
            return Ok(true);
        }

        if self.buffer.borrow().is_faulty() {
            self.invalidate_futures();
            return Err(CommandErrors::ProtocolViolation);
        }

        Ok(false)
    }

    /// Takes the message mapped to the future
    /// None is returned in case if message has been already taken or message is not complete yet
    pub(crate) fn take_frame(&self, id: &Identity) -> Option<P::FrameType> {
        if self.current_series.borrow().deref() != &id.series {
            return None;
        }

        self.buffer.borrow_mut().take_frame(id.index)
    }

    /// Takes and returns the next frame if existing.
    pub(crate) fn take_next_frame(&self) -> Option<P::FrameType> {
        self.buffer.borrow_mut().take_next_frame()
    }

    /// Invalidates all current futures after a fatal error.
    pub(crate) fn invalidate_futures(&self) {
        *self.current_series.borrow_mut() += 1;
        *self.next_index.borrow_mut() = 0;
        *self.clear_buffer.borrow_mut() = true;
    }

    /// Pending response was dropped before fully fetching response data
    pub(crate) fn drop_future(&self, id: Identity) {
        self.dropped_futures.borrow_mut().push(id);
    }

    /// Drops response data of dropped futures
    pub fn handle_dropped_futures(&self) {
        if self.dropped_futures.borrow().is_empty() {
            return;
        }

        let mut buffer = self.buffer.borrow_mut();

        self.dropped_futures.borrow_mut().retain(|id| {
            // Pending response got invalidated in the meanwhile
            if &id.series != self.current_series.borrow().deref() {
                return false;
            }

            // Clearing response data
            if buffer.is_complete(id.index) {
                buffer.take_frame(id.index);
                return false;
            }

            true
        })
    }

    /// Returns true if there are any remaining dropped futures
    pub fn remaining_dropped_futures(&self) -> bool {
        !self.dropped_futures.borrow().is_empty()
    }

    /// Clears buffered socket data.
    ///
    /// Unlike the non-blocking API, `embedded-nal-async` has no operation for draining only
    /// currently pending data without waiting. Reads which have not completed are expected to be
    /// cancellation-safe, so only data already received by this type has to be discarded here.
    fn clear_socket(&self) {
        self.buffer.borrow_mut().clear();
    }

    pub fn get_protocol(&self) -> P {
        self.protocol.clone()
    }

    #[cfg(test)]
    pub fn get_dropped_future_count(&self) -> usize {
        self.dropped_futures.borrow().len()
    }

    #[cfg(test)]
    pub fn get_pending_frame_count(&self) -> usize {
        self.buffer.borrow().pending_frame_count()
    }
}

impl<T: Read + Write, P: Protocol> Debug for Network<T, P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Network").finish()
    }
}
