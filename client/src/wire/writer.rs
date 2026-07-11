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
use super::{BytePos, WireType};

struct Writer {
    output: Vec<u8>,
    pos: BytePos,
}


impl Writer {

    pub fn new() -> Self {
        let output = Vec::new();
        let pos = BytePos(0);
        Writer {output, pos}
    }

    fn pos(&self) -> BytePos {
        self.pos
    }

    fn bytes(&self) -> &[u8] {
        &self.output
    }

    fn write_byte(&mut self, n: u8) {
        self.output.push(n);
        self.pos.0 += 1;
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
}




#[cfg(test)]
mod tests {
    use crate::wire::writer::Writer;

    #[test]
    fn write_varint_2_bytes() {
        let mut w = Writer::new();
        w.write_varint(150);
        assert_eq!(w.pos().0, 2);
        assert_eq!(w.bytes(), [0b1001_0110, 0b0000_0001]);
    }

    #[test]
    fn write_string() {
        let mut w = Writer::new();
        w.write_string_field(2, "testing");
        assert_eq!(w.bytes(),[0x12, 0x07, 0x74, 0x65, 0x73, 0x74, 0x69, 0x6e, 0x67]);
    }
}
