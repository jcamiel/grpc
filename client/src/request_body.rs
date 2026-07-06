use crate::client::RunnerError;
use serde_json::Value;
use std::fmt;
use std::fmt::Formatter;

#[derive(Debug)]
pub struct RequestBody {
    bytes: Vec<u8>,
}

pub enum RequestBodyError {
    InvalidJson { error: String },
}

impl fmt::Display for RequestBodyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RequestBodyError::InvalidJson { error } => {
                write!(f, "invalid request body, {error}")
            }
        }
    }
}

impl RequestBody {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, RequestBodyError> {
        let bytes = bytes.trim_ascii();
        let body = if bytes.is_empty() {
            let v: Value =
                serde_json::from_slice(bytes).map_err(|e| RequestBodyError::InvalidJson {
                    error: e.to_string(),
                })?;
            vec![]
        } else {
            vec![]
        };
        let request_body = RequestBody { bytes: body };
        Ok(request_body)
    }
}
