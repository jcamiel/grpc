/*
 * Hurl (https://hurl.dev)
 * Copyright (C) 2026 Orange
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *          http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */
use super::WireType;

pub struct Writer {
    output: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        let output = Vec::new();
        Writer { output }
    }

    fn len(&self) -> usize {
        self.output.len()
    }

    pub fn bytes(&self) -> &[u8] {
        &self.output
    }

    fn write_byte(&mut self, n: u8) {
        self.output.push(n);
    }

    fn write_bytes(&mut self, value: &[u8]) {
        for &b in value {
            self.write_byte(b);
        }
    }

    pub fn write_varint(&mut self, mut n: u64) {
        while n >= 0x80 {
            self.write_byte((n as u8) | 0x80);
            n >>= 7;
        }
        self.write_byte(n as u8);
    }
}

impl Writer {
    pub fn write_string_field(&mut self, number: u32, value: &str) {
        let tag = (number << 3) | WireType::Len as u32;
        self.write_varint(tag as u64);
        self.write_varint(value.len() as u64);
        self.write_bytes(value.as_bytes());
    }

    pub fn write_message_field(&mut self, number: u32, payload: &[u8]) {
        let tag = (number << 3) | WireType::Len as u32;
        self.write_varint(tag as u64);
        self.write_varint(payload.len() as u64);
        self.write_bytes(payload);
    }

    pub fn write_sfixed32_field(&mut self, number: u32, value: i32) {
        let tag = (number << 3) | WireType::I32 as u32;
        self.write_varint(tag as u64);
        self.write_bytes(&value.to_le_bytes());
    }

    pub fn write_int32_field(&mut self, number: u32, value: i32) {
        let tag = (number << 3) | WireType::VarInt as u32;
        self.write_varint(tag as u64);
        // Sign-extend i32 → i64, then reinterpret as u64. Negative values therefore encode
        // as a 10-byte varint (full two's-complement u64),
        self.write_varint(value as i64 as u64);
    }

    pub fn write_sint32_field(&mut self, number: u32, value: i32) {
        let tag = (number << 3) | WireType::VarInt as u32;
        self.write_varint(tag as u64);
        // Zigzag-encode the signed value before the varint. Small negatives stay small on the wire
        // (unlike `int32`, where any negative is 10 bytes)
        let zigzag = ((value as u32) << 1) ^ ((value >> 31) as u32);
        self.write_varint(zigzag as u64);
    }

    pub fn begin_grpc_frame(&mut self) {
        // Reserve the 5-byte gRPC Length-Prefixed-Message header at offset 0.
        // The length will be patched by [`Self::end_grpc_frame`].
        self.write_byte(0);
        self.write_bytes(&[0; 4]);
    }

    pub fn end_grpc_frame(&mut self) {
        let payload_len = (self.len() - 5) as u32;
        self.output[1..5].copy_from_slice(&payload_len.to_be_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::Writer;

    #[test]
    fn write_varint_2_bytes() {
        let mut w = Writer::new();
        w.write_varint(150);
        assert_eq!(w.len(), 2);
        assert_eq!(w.bytes(), [0b1001_0110, 0b0000_0001]);
    }

    #[test]
    fn write_string() {
        let mut w = Writer::new();
        w.write_string_field(2, "testing");
        assert_eq!(
            w.bytes(),
            [0x12, 0x07, 0x74, 0x65, 0x73, 0x74, 0x69, 0x6e, 0x67]
        );
    }

    #[test]
    fn grpc_frame_wraps_payload() {
        let mut w = Writer::new();
        w.begin_grpc_frame();
        w.write_string_field(1, "bob");
        w.end_grpc_frame();
        assert_eq!(
            w.bytes(),
            [0x00, 0x00, 0x00, 0x00, 0x05, 0x0a, 0x03, 0x62, 0x6f, 0x62]
        );
    }

    #[test]
    fn write_sfixed32() {
        let mut w = Writer::new();
        w.write_sfixed32_field(1, -1);
        assert_eq!(w.bytes(), [0x0d, 0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn write_int32_positive() {
        let mut w = Writer::new();
        w.write_int32_field(1, 150);
        assert_eq!(w.bytes(), [0x08, 0x96, 0x01]);
    }

    #[test]
    fn write_int32_negative_is_ten_bytes() {
        let mut w = Writer::new();
        w.write_int32_field(1, -1);
        assert_eq!(
            w.bytes(),
            [
                0x08, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01
            ]
        );
    }

    #[test]
    fn write_sint32_negative_uses_zigzag() {
        // -1 zigzags to 1, so a single payload byte — contrast with the
        // 10-byte int32 encoding of the same value.
        let mut w = Writer::new();
        w.write_sint32_field(1, -1);
        assert_eq!(w.bytes(), [0x08, 0x01]);
    }
}
