//! Bit-buffer reader for codestream parsing.
//!
//! Port of `ojph_bitbuffer_read.h`. Handles JPEG 2000 byte-stuffing:
//! after a 0xFF byte, the next byte's MSB is a stuffed bit.

/// Bit-level reader with JPEG 2000 0xFF byte-unstuffing.
///
/// Reads from a byte slice, automatically handling the rule that after
/// a 0xFF byte the MSB of the next byte is ignored (stuffed).
#[derive(Debug)]
pub struct BitBufferRead<'a> {
    data: &'a [u8],
    pos: usize,
    /// Accumulated bit buffer (left-justified).
    buf: u64,
    /// Number of valid bits in `buf`.
    bits_left: u32,
    /// True if the previous byte was 0xFF (triggers unstuffing).
    unstuff: bool,
}

impl<'a> BitBufferRead<'a> {
    /// Create a new bit reader over the given byte slice.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            buf: 0,
            bits_left: 0,
            unstuff: false,
        }
    }

    /// Fill the internal buffer with up to 32 new bits from the byte stream.
    pub fn fill(&mut self) {
        while self.bits_left <= 32 && self.pos < self.data.len() {
            let byte = self.data[self.pos] as u64;
            self.pos += 1;
            let bits_to_add = if self.unstuff { 7 } else { 8 };
            let val = if self.unstuff { byte & 0x7F } else { byte };
            self.buf |= val << (56 - self.bits_left - (8 - bits_to_add));
            self.bits_left += bits_to_add;
            self.unstuff = (self.data[self.pos - 1]) == 0xFF;
        }
    }

    /// Peek at the top `n` bits without consuming them (n <= 32).
    #[inline]
    pub fn peek(&self, n: u32) -> u32 {
        debug_assert!(n <= 32 && n <= self.bits_left);
        (self.buf >> (64 - n)) as u32
    }

    /// Consume `n` bits from the buffer.
    #[inline]
    pub fn advance(&mut self, n: u32) {
        debug_assert!(n <= self.bits_left);
        self.buf <<= n;
        self.bits_left -= n;
    }

    /// Read `n` bits and return them as a u32 (n <= 32).
    #[inline]
    pub fn read(&mut self, n: u32) -> u32 {
        if self.bits_left < n {
            self.fill();
        }
        let val = self.peek(n);
        self.advance(n);
        val
    }

    /// Returns the number of valid bits currently in the buffer.
    #[inline]
    pub fn available_bits(&self) -> u32 {
        self.bits_left
    }

    /// Returns the current byte position in the data stream.
    #[inline]
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Returns true if the previous byte read was 0xFF.
    #[inline]
    pub fn is_unstuffing(&self) -> bool {
        self.unstuff
    }

    /// Reset the reader with new data.
    pub fn reset(&mut self, data: &'a [u8]) {
        self.data = data;
        self.pos = 0;
        self.buf = 0;
        self.bits_left = 0;
        self.unstuff = false;
    }
}
