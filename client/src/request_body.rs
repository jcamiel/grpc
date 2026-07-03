#[derive(Debug)]
pub struct RequestBody {
    bytes: Vec<u8>,
}

impl RequestBody {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn from_bytes(_bytes: &[u8]) -> Self {
        let bytes = vec![];
        RequestBody { bytes }
    }
}
