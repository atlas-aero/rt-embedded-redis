use crate::network::protocol::Protocol;
use alloc::vec;
use alloc::vec::Vec;
use bytes::Bytes;
use heapless::Vec as HVec;

/// Buffer for unparsed/incomplete + parsed/complete frames
/// FRAME_SIZE: Max. number of parsed but not yet handled frames. Basically this defines the max. number of parallel futures.
pub(crate) struct ResponseBuffer<P: Protocol, const FRAME_SIZE: usize> {
    decoder: P,

    /// Unparsed data buffer
    buffer: Vec<u8>,

    /// Parsed frames
    frames: HVec<Option<P::FrameType>, FRAME_SIZE>,

    /// Number of non taken messages in message vector
    frame_count: usize,

    /// Frame index offset on external access (e.g. complete() or take_message())
    /// So each new message gets an unique external index, while we can drain frame vector when
    /// alle messages are taken
    frame_offset: usize,

    /// Received unknown message prefix
    faulty: bool,
}

impl<P: Protocol, const FRAME_SIZE: usize> ResponseBuffer<P, FRAME_SIZE> {
    pub fn new(protocol: P) -> ResponseBuffer<P, FRAME_SIZE> {
        Self {
            decoder: protocol,
            buffer: vec![],
            frames: HVec::new(),
            frame_count: 0,
            frame_offset: 0,
            faulty: false,
        }
    }

    /// Appends data to buffer
    pub fn append(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
        self.parse_frames();
    }

    /// Takes the frame at the given index
    pub fn take_frame(&mut self, mut index: usize) -> Option<P::FrameType> {
        // Invalid index given
        if index < self.frame_offset {
            return None;
        }

        index -= self.frame_offset;

        // Message does not (yet) exist
        if self.frames.len() <= index {
            return None;
        }

        let frame = self.frames[index].take();
        if frame.is_some() {
            self.frame_count -= 1;
        }

        if self.frame_count == 0 {
            self.frame_offset += self.frames.len();
            self.frames.clear();
        }

        frame
    }

    /// Parses buffer and extracts messages
    /// Buffer is drained to only contain non-complete messages
    fn parse_frames(&mut self) {
        // Start of next message
        let mut start = 0;

        // Position of last termination in buffer
        let mut last_termination = None;

        while !self.faulty {
            let termination = self.parse_frame(start);
            // No more frames left
            if termination.is_none() {
                break;
            }

            start = termination.unwrap() + 1;
            last_termination = termination;
        }

        // No message was found, so buffer can stay unchanged
        if last_termination.is_none() {
            return;
        }

        // No unparsed data remaining in buffer
        if (last_termination.unwrap() + 1) == self.buffer.len() {
            return self.buffer.clear();
        }

        self.buffer.drain(..=last_termination.unwrap());
    }

    /// Parses the next frame
    ///
    /// # Arguments
    ///
    /// * `start`: Start parsing at this position
    ///
    /// returns: Option<usize> Position/Index of last termination
    /// None is returned in case no message was found
    fn parse_frame(&mut self, start: usize) -> Option<usize> {
        if start >= self.buffer.len() {
            return None;
        }

        let bytes = Bytes::from(self.buffer[start..].to_vec());

        let result = self.decoder.decode(&bytes);
        if result.is_err() {
            self.faulty = true;
            return None;
        }

        // No frame found
        if result.as_ref().unwrap().is_none() {
            return None;
        }

        let frame = result.unwrap().unwrap();
        let _ = self.frames.push(Some(frame.0));
        self.frame_count += 1;
        Some(frame.1 - 1 + start)
    }

    /// Is message at index given index complete
    pub fn is_complete(&self, mut index: usize) -> bool {
        if index < self.frame_offset {
            return false;
        }

        index -= self.frame_offset;
        self.frames.len() > index
    }

    /// If true, an protocol violation was detected
    /// Since the cause (e.g. Redis bug, network fault, etc.) is unclear, this is a fatal problem.
    /// The mapping of messages indexes can no longer be guaranteed from this point on.
    pub fn is_faulty(&self) -> bool {
        self.faulty
    }

    /// Resets the buffer in case of fatal error
    pub fn clear(&mut self) {
        self.frames.clear();
        self.buffer.clear();
        self.frame_offset = 0;
        self.frame_count = 0;
        self.faulty = false;
    }

    #[cfg(test)]
    pub fn pending_frame_count(&self) -> usize {
        self.frame_count
    }

    #[cfg(test)]
    pub fn frame_offset(&self) -> usize {
        self.frame_offset
    }
}
