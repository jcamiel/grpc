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
    /// Creates a new writer.
    pub fn new() -> Self {
        let output = Vec::new();
        Writer { output }
    }

    /// Returns the number of written bytes.
    fn len(&self) -> usize {
        self.output.len()
    }

    /// Returns the written bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.output
    }

    /// Writes a single byte.
    fn write_byte(&mut self, n: u8) {
        self.output.push(n);
    }

    /// Writes some bytes.
    fn write_bytes(&mut self, value: &[u8]) {
        for &b in value {
            self.write_byte(b);
        }
    }

    /// Writes a varint.
    pub fn write_varint(&mut self, mut n: u64) {
        while n >= 0x80 {
            self.write_byte((n as u8) | 0x80);
            n >>= 7;
        }
        self.write_byte(n as u8);
    }
}

impl Writer {
    /// Writes a protobuf string field.
    pub fn write_string_field(&mut self, number: u32, value: &str) {
        let tag = (number << 3) | WireType::Len as u32;
        self.write_varint(tag as u64);
        self.write_varint(value.len() as u64);
        self.write_bytes(value.as_bytes());
    }

    /// Writes a protobuf message field (payload is the encoded bytes).
    pub fn write_message_field(&mut self, number: u32, payload: &[u8]) {
        let tag = (number << 3) | WireType::Len as u32;
        self.write_varint(tag as u64);
        self.write_varint(payload.len() as u64);
        self.write_bytes(payload);
    }

    /// Writes a sfixed32 integer field.
    pub fn write_sfixed32_field(&mut self, number: u32, value: i32) {
        let tag = (number << 3) | WireType::I32 as u32;
        self.write_varint(tag as u64);
        self.write_bytes(&value.to_le_bytes());
    }

    /// Writes a int32 integer field.
    pub fn write_int32_field(&mut self, number: u32, value: i32) {
        let tag = (number << 3) | WireType::VarInt as u32;
        self.write_varint(tag as u64);
        // Sign-extend i32 → i64, then reinterpret as u64. Negative values therefore encode
        // as a 10-byte varint (full two's-complement u64),
        self.write_varint(value as i64 as u64);
    }

    /// Writes a sint32 integer field.
    pub fn write_sint32_field(&mut self, number: u32, value: i32) {
        let tag = (number << 3) | WireType::VarInt as u32;
        self.write_varint(tag as u64);
        // Zigzag-encode the signed value before the varint. Small negatives stay small on the wire
        // (unlike `int32`, where any negative is 10 bytes)
        let zigzag = ((value as u32) << 1) ^ ((value >> 31) as u32);
        self.write_varint(zigzag as u64);
    }

    /// Writes a boolean field.
    pub fn write_bool_field(&mut self, number: u32, value: bool) {
        let tag = (number << 3) | WireType::VarInt as u32;
        self.write_varint(tag as u64);
        // `bool as u64` is defined to be 0 or 1, so this is the whole encoding.
        self.write_varint(value as u64);
    }

    /// Writes a uint32 integer field.
    pub fn write_uint32_field(&mut self, number: u32, value: u32) {
        let tag = (number << 3) | WireType::VarInt as u32;
        self.write_varint(tag as u64);
        // Straight varint of the widened u32; no sign extension needed since unsigned.
        self.write_varint(value as u64);
    }

    /// Writes a fixed32 integer field.
    pub fn write_fixed32_field(&mut self, number: u32, value: u32) {
        let tag = (number << 3) | WireType::I32 as u32;
        self.write_varint(tag as u64);
        self.write_bytes(&value.to_le_bytes());
    }

    /// Writes a sfixed64 integer field.
    pub fn write_sfixed64_field(&mut self, number: u32, value: i64) {
        let tag = (number << 3) | WireType::I64 as u32;
        self.write_varint(tag as u64);
        self.write_bytes(&value.to_le_bytes());
    }

    /// Writes the begin of a gRPC frame.
    pub fn begin_grpc_frame(&mut self) {
        // Reserve the 5-byte gRPC Length-Prefixed-Message header at offset 0.
        // The length will be patched by [`Self::end_grpc_frame`].
        self.write_byte(0);
        self.write_bytes(&[0; 4]);
    }

    /// Writes the end of a gRPC frame.
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

    #[test]
    fn write_bool() {
        let mut w = Writer::new();
        w.write_bool_field(1, true);
        assert_eq!(w.bytes(), [0x08, 0x01]);
    }

    #[test]
    fn write_uint32() {
        let mut w = Writer::new();
        w.write_uint32_field(1, 300);
        // tag 0x08, then varint 300 = 0xac 0x02
        assert_eq!(w.bytes(), [0x08, 0xac, 0x02]);
    }

    #[test]
    fn write_fixed32() {
        let mut w = Writer::new();
        w.write_fixed32_field(1, 1);
        // tag 0x0d (I32 wire type), then LE bytes of 1
        assert_eq!(w.bytes(), [0x0d, 0x01, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn write_sfixed64() {
        let mut w = Writer::new();
        w.write_sfixed64_field(1, -1);
        // tag 0x09 (I64 wire type = 1, field 1), then 8 bytes LE two's-complement -1
        assert_eq!(
            w.bytes(),
            [0x09, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
        );
    }
}
