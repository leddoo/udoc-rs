#[derive(Clone)]
pub struct Reader<'buf, T> {
    pub buffer: &'buf [T],
    pub cursor: usize,
}

#[allow(dead_code)]
impl<'rdr, T> Reader<'rdr, T> {
    pub fn new(buffer: &'rdr [T]) -> Reader<'rdr, T> {
        Reader { buffer: buffer, cursor: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.buffer.len() - self.cursor
    }

    pub fn rest(&self) -> &'rdr [T] {
        &self.buffer[self.cursor..]
    }


    pub fn empty(&self) -> bool {
        !self.has_some()
    }

    pub fn has_some(&self) -> bool {
        self.cursor < self.buffer.len()
    }

    pub fn peek(&self, offset: usize) -> Option<&'rdr T> {
        self.buffer.get(self.cursor + offset)
    }

    pub fn next(&mut self) -> Option<&'rdr T> {
        self.peek(0).map(|result| {
            self.cursor += 1;
            result
        })
    }


    pub fn has_n(&self, n: usize) -> bool {
        self.cursor + n <= self.buffer.len()
    }

    pub fn peek_n(&self, n: usize) -> Option<&'rdr [T]> {
        if self.has_n(n) {
            return Some(&self.buffer[self.cursor .. self.cursor + n]);
        }
        None
    }

    pub fn next_n(&mut self, n: usize) -> Option<&'rdr [T]> {
        self.peek_n(n).map(|result| {
            self.cursor += n;
            result
        })
    }
}


impl<'rdr> Reader<'rdr, u8> {
    pub fn next_bytes<const N: usize>(&mut self) -> Option<[u8; N]> {
        let mut bytes = [0; N];
        bytes.copy_from_slice(self.next_n(N)?);
        Some(bytes)
    }

    pub fn next_bytes_le<const N: usize>(&mut self) -> Option<[u8; N]> {
        #[cfg(target_endian = "little")] {
            self.next_bytes::<N>()
        }
        #[cfg(not(target_endian = "little"))] {
            self.next_bytes::<N>().map(|mut bytes| { bytes.reverse(); bytes })
        }
    }

    /*
    pub fn next_bytes_be<const N: usize>(&mut self) -> Option<[u8; N]> {
        #[cfg(target_endian = "little")] {
            self.next_bytes::<N>().map(|mut bytes| { bytes.reverse(); bytes })
        }
        #[cfg(not(target_endian = "little"))] {
            self.next_bytes::<N>()
        }
    }
    */

    /*
    pub fn next_u8(&mut self)  -> Option<u8>  { self.next_bytes::<1>().map(u8::from_ne_bytes) }
    pub fn next_u16(&mut self) -> Option<u16> { self.next_bytes::<2>().map(u16::from_ne_bytes) }
    pub fn next_u32(&mut self) -> Option<u32> { self.next_bytes::<4>().map(u32::from_ne_bytes) }
    pub fn next_u64(&mut self) -> Option<u64> { self.next_bytes::<8>().map(u64::from_ne_bytes) }

    pub fn next_i8(&mut self)  -> Option<i8>  { self.next_bytes::<1>().map(i8::from_ne_bytes) }
    pub fn next_i16(&mut self) -> Option<i16> { self.next_bytes::<2>().map(i16::from_ne_bytes) }
    pub fn next_i32(&mut self) -> Option<i32> { self.next_bytes::<4>().map(i32::from_ne_bytes) }
    pub fn next_i64(&mut self) -> Option<i64> { self.next_bytes::<8>().map(i64::from_ne_bytes) }

    pub fn next_f32(&mut self) -> Option<f32> { self.next_bytes::<4>().map(f32::from_ne_bytes) }
    pub fn next_f64(&mut self) -> Option<f64> { self.next_bytes::<8>().map(f64::from_ne_bytes) }
    */


    pub fn next_u8_le(&mut self)  -> Option<u8>  { self.next_bytes::<1>().map(u8::from_le_bytes) }
    pub fn next_u16_le(&mut self) -> Option<u16> { self.next_bytes::<2>().map(u16::from_le_bytes) }
    pub fn next_u32_le(&mut self) -> Option<u32> { self.next_bytes::<4>().map(u32::from_le_bytes) }
    pub fn next_u64_le(&mut self) -> Option<u64> { self.next_bytes::<8>().map(u64::from_le_bytes) }

    pub fn next_i8_le(&mut self)  -> Option<i8>  { self.next_bytes::<1>().map(i8::from_le_bytes) }
    pub fn next_i16_le(&mut self) -> Option<i16> { self.next_bytes::<2>().map(i16::from_le_bytes) }
    pub fn next_i32_le(&mut self) -> Option<i32> { self.next_bytes::<4>().map(i32::from_le_bytes) }
    pub fn next_i64_le(&mut self) -> Option<i64> { self.next_bytes::<8>().map(i64::from_le_bytes) }

    pub fn next_f32_le(&mut self) -> Option<f32> { self.next_bytes::<4>().map(f32::from_le_bytes) }
    pub fn next_f64_le(&mut self) -> Option<f64> { self.next_bytes::<8>().map(f64::from_le_bytes) }


    /*
    pub fn next_u8_be(&mut self)  -> Option<u8>  { self.next_bytes::<1>().map(u8::from_be_bytes) }
    pub fn next_u16_be(&mut self) -> Option<u16> { self.next_bytes::<2>().map(u16::from_be_bytes) }
    pub fn next_u32_be(&mut self) -> Option<u32> { self.next_bytes::<4>().map(u32::from_be_bytes) }
    pub fn next_u64_be(&mut self) -> Option<u64> { self.next_bytes::<8>().map(u64::from_be_bytes) }

    pub fn next_i8_be(&mut self)  -> Option<i8>  { self.next_bytes::<1>().map(i8::from_be_bytes) }
    pub fn next_i16_be(&mut self) -> Option<i16> { self.next_bytes::<2>().map(i16::from_be_bytes) }
    pub fn next_i32_be(&mut self) -> Option<i32> { self.next_bytes::<4>().map(i32::from_be_bytes) }
    pub fn next_i64_be(&mut self) -> Option<i64> { self.next_bytes::<8>().map(i64::from_be_bytes) }

    pub fn next_f32_be(&mut self) -> Option<f32> { self.next_bytes::<4>().map(f32::from_be_bytes) }
    pub fn next_f64_be(&mut self) -> Option<f64> { self.next_bytes::<8>().map(f64::from_be_bytes) }
    */
}

