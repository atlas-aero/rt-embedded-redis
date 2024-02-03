pub mod auth;
pub mod bgsave;
pub mod builder;
pub mod custom;
pub mod get;
pub mod hello;
pub mod helpers;
pub mod hget;
pub mod hset;
pub mod ping;
pub mod publish;
pub mod set;
#[cfg(test)]
pub(crate) mod tests;

/// Error in case Redis response type does not match specification
#[derive(Debug)]
pub struct ResponseTypeError {}

/// Generic command structure. F is either [Resp2Frame](redis_protocol::resp2::types::Frame) or
/// [Resp3Frame](redis_protocol::resp3::types::Frame)
pub trait Command<F> {
    /// Response type, either a custom evaluated "high-level" response or the original RESP frame
    type Response;

    /// Encodes the command to RESP2/RESP3 frame
    fn encode(&self) -> F;

    /// The command has the ability to evaluate the response frame and craft its own high level
    /// response from that.
    /// Its also possible to just return 1:1 the RESP2 frame.
    ///
    /// Error responses are captured upfront and converted to CommandErrors::ErrorResponse.
    /// So error responses never reach that method.
    ///
    /// Returns Error only in case of protocol violation (e.g. received an array for an command
    /// that only returns strings)
    fn eval_response(&self, frame: F) -> Result<Self::Response, ResponseTypeError>;
}
