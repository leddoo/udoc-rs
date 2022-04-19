use slice_reader::{Reader, byte_order::aliases::LE};
use crate::{wire_type::*, utils::*};


pub fn decode_size_prefixed<'val>(reader: &mut Reader<'val, u8>) -> Option<&'val [u8]> {
    let size = decode_size_as_usize::<LE>(reader)?;
    reader.next_n(size)
}

pub fn decode_length_prefixed(buffer: &[u8]) -> Option<(usize, Reader<u8>)> {
    let mut reader = Reader::new(buffer);
    let length =
        if reader.has_some() { decode_size_as_usize::<LE>(&mut reader)? }
        else                 { 0 };
    Some((length, reader))
}


pub fn decode_symbol<'val>(reader: &mut Reader<'val, u8>) -> Option<&'val [u8]> {
    let size = decode_size::<LE>(reader)?;
    let (size, is_bytes) = (size >> 1, size & 1 != 0);
    if is_bytes {
        reader.next_n(u64_to_usize(size)?)
    }
    else {
        // reserved.
        None
    }
}



#[derive(Clone, Copy)]
pub struct Header {
    pub wire_type: WireType,
    pub has_kind:  bool,
    pub has_tags:  bool,
}

pub fn decode_header(reader: &mut Reader<u8>) -> Option<Header> {
    let header = reader.next_u8_le()?;
    Some(Header {
        wire_type: WireType::from_u8(header & WIRE_TYPE_MASK)?,
        has_kind: header & WIRE_FLAG_KIND != 0,
        has_tags: header & WIRE_FLAG_TAGS != 0,
    })
}


pub fn decode_kind<'val>(has_kind: bool, reader: &mut Reader<'val, u8>) -> Option<&'val [u8]> {
    if has_kind {
        decode_symbol(reader)
    }
    else {
        Some(&reader.buffer[0..0])
    }
}


pub fn decode_tags<'val>(has_tags: bool, reader: &mut Reader<'val, u8>) -> Option<&'val [u8]> {
    if has_tags {
        decode_size_prefixed(reader)
    }
    else {
        Some(&reader.buffer[0..0])
    }
}

pub fn decode_tag<'val>(reader: &mut Reader<'val, u8>) -> Option<(&'val [u8], Value<'val>)> {
    Some((decode_symbol(reader)?, decode_value(reader)?))
}


pub enum Payload<'val> {
    Null,
    Bool      (bool),
    Nat       (&'val [u8]),
    Nat8      (u8),
    Nat16     (u16),
    Nat32     (u32),
    Nat64     (u64),
    Int       (&'val [u8]),
    Int8      (i8),
    Int16     (i16),
    Int32     (i32),
    Int64     (i64),
    Float32   (f32),
    Float64   (f64),
    Decimal32 ([u8; 4]),
    Decimal64 ([u8; 8]),
    Bytes     (&'val [u8]),
    String    (&'val [u8]),
    Symbol    (&'val [u8]),
    List      (&'val [u8]),
}

pub fn decode_payload<'val>(ty: WireType, reader: &mut Reader<'val, u8>) -> Option<Payload<'val>> {
    use WireType::*;
    Some(match ty {
        Null      => { Payload::Null },
        BoolFalse => { Payload::Bool(false) },
        BoolTrue  => { Payload::Bool(true) },
        Nat8      => { Payload::Nat8(reader.next_u8_le()?) },
        Nat16     => { Payload::Nat16(reader.next_u16_le()?) },
        Nat32     => { Payload::Nat32(reader.next_u32_le()?) },
        Nat64     => { Payload::Nat64(reader.next_u64_le()?) },
        Int8      => { Payload::Int8(reader.next_i8_le()?) },
        Int16     => { Payload::Int16(reader.next_i16_le()?) },
        Int32     => { Payload::Int32(reader.next_i32_le()?) },
        Int64     => { Payload::Int64(reader.next_i64_le()?) },
        Float32   => { Payload::Float32(reader.next_f32_le()?) },
        Float64   => { Payload::Float64(reader.next_f64_le()?) },
        Decimal32 => { Payload::Decimal32(reader.next_bytes_endian::<4, LE>()?) },
        Decimal64 => { Payload::Decimal64(reader.next_bytes_endian::<8, LE>()?) },
        Nat       => { Payload::Nat(decode_size_prefixed(reader)?) },
        Int       => { Payload::Int(decode_size_prefixed(reader)?) },
        Bytes     => { Payload::Bytes(decode_size_prefixed(reader)?) },
        String    => { Payload::String(decode_size_prefixed(reader)?) },
        Symbol    => { Payload::Symbol(decode_symbol(reader)?) },
        List      => { Payload::List(decode_size_prefixed(reader)?) },
    })
}


pub struct Value<'val> {
    pub header:  Header,
    pub kind:    &'val [u8],
    pub tags:    &'val [u8],
    pub payload: Payload<'val>,
}

impl<'val> Value<'val> {
    pub fn tags(&self) -> Option<TagDecoder> {
        TagDecoder::new(self.tags)
    }
}

pub fn decode_value<'rdr>(reader: &mut Reader<'rdr, u8>) -> Option<Value<'rdr>> {
    let header = decode_header(reader)?;
    Some(Value {
        header,
        kind:    decode_kind(header.has_kind, reader)?,
        tags:    decode_tags(header.has_tags, reader)?,
        payload: decode_payload(header.wire_type, reader)?,
    })
}




pub struct TagDecoder<'val> {
    pub remaining: usize,
    pub reader:    Reader<'val, u8>,
}

impl<'val> TagDecoder<'val> {
    pub fn new(tags: &'val [u8]) -> Option<TagDecoder> {
        let (remaining, reader) = decode_length_prefixed(tags)?;
        if reader.remaining() < 2*remaining {
            return None;
        }
        Some(TagDecoder { remaining, reader })
    }

    pub fn check_error(self) -> Result<(), ()> {
        if self.remaining != 0 || self.reader.has_some() {
            return Err(());
        }
        Ok(())
    }
}

impl<'val> Iterator for TagDecoder<'val> {
    type Item = (&'val [u8], Value<'val>);

    fn next(&mut self) -> Option<(&'val [u8], Value<'val>)> {
        if self.remaining > 0 {
            let result = decode_tag(&mut self.reader)?;
            // remaining != 0 => error.
            self.remaining -= 1;
            return Some(result)
        }
        None
    }
}


pub struct ListDecoder<'val> {
    pub remaining: usize,
    pub reader:    Reader<'val, u8>,
}

impl<'val> ListDecoder<'val> {
    pub fn new(payload: &'val [u8]) -> Option<ListDecoder> {
        let (remaining, reader) = decode_length_prefixed(payload)?;
        if reader.remaining() < 1*remaining {
            return None;
        }
        Some(ListDecoder { remaining, reader })
    }

    pub fn check_error(self) -> Result<(), ()> {
        if self.remaining != 0 || self.reader.has_some() {
            return Err(());
        }
        Ok(())
    }
}

impl<'val> Iterator for ListDecoder<'val> {
    type Item = Value<'val>;

    fn next(&mut self) -> Option<Value<'val>> {
        if self.remaining > 0 {
            let result = decode_value(&mut self.reader)?;
            // remaining != 0 => error.
            self.remaining -= 1;
            return Some(result)
        }
        None
    }
}

