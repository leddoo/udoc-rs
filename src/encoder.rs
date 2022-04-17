use slice_reader::Reader;
use crate::utils::*;


#[derive(Debug, Clone)]
pub enum Error {
    SizeOverflow,
}


pub struct Encoder {
    buffer: Vec<u8>,
    sizers: Vec<Sizer>,

    size_offsets: Vec<u8>, // relative, encoded with udoc size encoding.
    last_size_offset: usize,

    size_max_bytes: usize,
    compress_sizes: bool,
    size_overflow: bool,
}

impl Encoder {
    pub fn new(size_max_bytes: usize, compress_sizes: bool) -> Encoder {
        match size_max_bytes { 1 | 2 | 4 | 8 => (), _ => unreachable!() }

        Encoder {
            buffer: vec![],
            sizers: vec![ Sizer { offset: 0, size: 0 } ],

            size_offsets: vec![],
            last_size_offset: 0,

            size_max_bytes,
            compress_sizes,
            size_overflow: false,
        }
    }

    fn commit_size(&mut self, size: usize) {
        self.sizers.last_mut().unwrap().size += size;
    }

    pub fn append(&mut self, bytes: &[u8]) {
        self.buffer.extend(bytes);
        self.commit_size(bytes.len());
    }

    pub fn append_byte(&mut self, byte: u8) {
        self.buffer.push(byte);
        self.commit_size(1);
    }

    pub fn append_size(&mut self, value: u64) {
        let (bytes, length) = encode_size(value);
        self.append(&bytes[..length]);
    }

    pub fn append_symbol(&mut self, symbol: &[u8]) {
        let (bytes, length) = encode_size((symbol.len() << 1 | 1) as u64);
        self.append(&bytes[..length]);
        self.append(symbol);
    }


    pub fn begin_size(&mut self) {
        let offset = self.buffer.len();
        self.buffer.extend((0..self.size_max_bytes).map(|_| 0));
        self.sizers.push(Sizer { offset: offset, size: 0 });

        if self.compress_sizes {
            let delta = (offset - self.last_size_offset) as u64;
            let (delta, length) = encode_size(delta);
            self.size_offsets.extend(&delta[..length]);
            self.last_size_offset = offset;
        }
    }

    pub fn end_size(&mut self) {
        assert!(self.sizers.len() > 1);
        let sizer = self.sizers.pop().unwrap();

        let (size, length) = encode_size(sizer.size as u64);
        if length > self.size_max_bytes {
            self.size_overflow = true;
        }

        let offset = sizer.offset;
        match self.size_max_bytes {
            1 => self.buffer[offset..offset + 1].copy_from_slice(&size[0..1]),
            2 => self.buffer[offset..offset + 2].copy_from_slice(&size[0..2]),
            4 => self.buffer[offset..offset + 4].copy_from_slice(&size[0..4]),
            8 => self.buffer[offset..offset + 8].copy_from_slice(&size[0..8]),
            _ => unreachable!()
        }

        if self.compress_sizes {
            self.commit_size(length + sizer.size);
        }
        else {
            self.commit_size(self.size_max_bytes + sizer.size);
        }
    }

    pub fn size(&self) -> usize {
        assert!(self.sizers.len() == 1);
        self.sizers[0].size
    }

    fn compress(&self, dest: &mut Vec<u8>) {
        let size = self.size();

        let old_length = dest.len();
        dest.reserve(size);

        let mut buffer  = Reader::new(&self.buffer);
        let mut offsets = Reader::new(&self.size_offsets);
        let mut first = true;

        while buffer.has_some() {
            if offsets.has_some() {
                let next_size = decode_size(&mut offsets).unwrap();

                let mut next_size = next_size as usize;
                if first {
                    first = false;
                }
                else {
                    // note: the offsets are the number of bytes between two
                    // sizes in the buffer. the previous size's bytes are
                    // already consumed.
                    next_size -= self.size_max_bytes;
                }

                dest.extend(buffer.next_n(next_size).unwrap());

                let (_size, length) = peek_decode_size(&buffer).unwrap();
                dest.extend(buffer.peek_n(length).unwrap());
                buffer.next_n(self.size_max_bytes).unwrap();
            }
            else {
                dest.extend(buffer.rest());
                break;
            }
        }
        assert_eq!(dest.len() - old_length, size);
    }

    pub fn build(self) -> Result<Vec<u8>, Error> {
        assert!(self.sizers.len() == 1);
        if self.size_overflow {
            return Err(Error::SizeOverflow);
        }

        if self.compress_sizes {
            let mut result = vec![];
            self.compress(&mut result);
            Ok(result)
        }
        else {
            Ok(self.buffer)
        }
    }

    pub fn build_append(&self, dest: &mut Vec<u8>) -> Result<(), Error> {
        assert!(self.sizers.len() == 1);
        if self.size_overflow {
            return Err(Error::SizeOverflow);
        }

        if self.compress_sizes {
            self.compress(dest);
        }
        else {
            dest.extend(&self.buffer);
        }
        Ok(())
    }
}

impl Default for Encoder {
    fn default() -> Encoder {
        Encoder::new(8, true)
    }
}

struct Sizer {
    offset: usize,
    size:   usize,
}

