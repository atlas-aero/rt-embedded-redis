use crate::network::client::CommandErrors;
use crate::network::future::Identity;
use crate::network::protocol::Protocol;
use crate::network::response::{MemoryParameters, ResponseBuffer};
use alloc::vec;
use alloc::vec::Vec;
use bytes::BytesMut;
use core::cell::RefCell;
use core::fmt::{Debug, Formatter};
use core::ops::{Deref, DerefMut};
use embedded_nal::TcpClientStack;

/// Manges interaction between network stack and response buffer
pub(crate) struct Network<'a, N: TcpClientStack, P: Protocol> {
    protocol: P,
    stack: RefCell<&'a mut N>,
    socket: RefCell<&'a mut N::TcpSocket>,
    buffer: RefCell<ResponseBuffer<P>>,

    /// Current valid Future series
    current_series: RefCell<usize>,

    /// Index of next Future
    next_index: RefCell<usize>,

    /// Indicates a pending buffer clearance on fatal errors
    clear_buffer: RefCell<bool>,

    /// List of dropped futures, which did not call wait()
    /// For not leaking memory, response data of this futures is dropped on next send() call
    dropped_futures: RefCell<Vec<Identity>>,
}

impl<'a, N: TcpClientStack, P: Protocol> Network<'a, N, P> {
    pub(crate) fn new(
        stack: RefCell<&'a mut N>,
        socket: RefCell<&'a mut N::TcpSocket>,
        protocol: P,
        memory: MemoryParameters,
    ) -> Self {
        Network {
            protocol: protocol.clone(),
            stack,
            socket,
            buffer: RefCell::new(ResponseBuffer::new(protocol, memory)),
            current_series: RefCell::new(0),
            next_index: RefCell::new(0),
            clear_buffer: RefCell::new(false),
            dropped_futures: RefCell::new(vec![]),
        }
    }

    /// Appends 32 byte to the given buffer
    pub(crate) fn receive_chunk(&self) -> nb::Result<(), N::Error> {
        let mut local_buffer: [u8; 32] = [0; 32];
        let mut stack = self.stack.borrow_mut();
        let mut socket = self.socket.borrow_mut();

        match stack.receive(socket.deref_mut(), &mut local_buffer) {
            Ok(byte_count) => {
                self.buffer.borrow_mut().append(&local_buffer[0..byte_count]);
                Ok(())
            }
            Err(error) => nb::Result::Err(error),
        }
    }

    /// Returns true if the memory limit is reached
    pub(crate) fn is_buffer_full(&self) -> bool {
        self.buffer.borrow().is_full()
    }

    /// Encodes and sends the given command
    pub(crate) fn send(&self, frame: P::FrameType) -> Result<Identity, CommandErrors> {
        // Seems a fata error invalidated the current series, so everything needs to be cleared
        if *self.clear_buffer.borrow().deref() {
            self.clear_socket();
            *self.clear_buffer.borrow_mut() = false;
        }

        // Handle dropped futures for not leaking memory
        self.handle_dropped_futures();

        self.send_frame(frame)?;

        let identity = Identity {
            series: *self.current_series.borrow(),
            index: *self.next_index.borrow(),
        };
        *self.next_index.borrow_mut() += 1;
        Ok(identity)
    }

    /// Raw network logic for sending a frame
    pub(crate) fn send_frame(&self, frame: P::FrameType) -> Result<(), CommandErrors> {
        let mut buffer = BytesMut::new();
        if self.protocol.encode_bytes(&mut buffer, &frame).is_err() {
            return Err(CommandErrors::EncodingCommandFailed);
        }

        let mut stack = self.stack.borrow_mut();
        let mut socket = self.socket.borrow_mut();

        if stack.send(socket.deref_mut(), buffer.as_ref()).is_err() {
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

    /// In case of fatal errors alle current futures are invalidated
    pub(crate) fn invalidate_futures(&self) {
        *self.current_series.borrow_mut() += 1;
        *self.next_index.borrow_mut() = 0;
        *self.clear_buffer.borrow_mut() = true;
    }

    /// Future was dropped before fully fetching response data
    pub(crate) fn drop_future(&self, id: Identity) {
        self.dropped_futures.borrow_mut().push(id);
    }

    /// Drops response data of dropped futures
    pub fn handle_dropped_futures(&self) {
        if self.dropped_futures.borrow().is_empty() {
            return;
        }

        self.receive_all();
        let mut buffer = self.buffer.borrow_mut();

        self.dropped_futures.borrow_mut().retain(|id| {
            // Future got invalidated in the meanwhile
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

    /// Receives all pending socket data
    pub fn receive_all(&self) {
        let mut result = Ok(());

        while result.is_ok() {
            result = self.receive_chunk();
        }
    }

    /// Clears buffer and pending socket data
    fn clear_socket(&self) {
        let mut stack = self.stack.borrow_mut();
        let mut socket = self.socket.borrow_mut();

        loop {
            let mut local_buffer: [u8; 32] = [0; 32];

            match stack.receive(socket.deref_mut(), &mut local_buffer) {
                Ok(_) => {}
                Err(_) => {
                    break;
                }
            }
        }

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

impl<'a, N: TcpClientStack, P: Protocol> Debug for Network<'a, N, P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Network").finish()
    }
}
