use slice_reader::Reader;
use byte_order::aliases::LE;
use crate::{wire_type::*};


pub fn u64_to_usize(value: u64) -> Option<usize> {
    value.try_into().ok()
}

pub fn decode_size(reader: &mut Reader<u8>) -> Option<u64> {
    crate::utils::decode_size::<LE>(reader)
}

pub fn decode_size_as_usize(reader: &mut Reader<u8>) -> Option<usize> {
    u64_to_usize(decode_size(reader)?)
}


pub enum TagSymbol<'val> {
    Bytes (&'val [u8]),
}

pub fn decode_tag_symbol<'val>(reader: &mut Reader<'val, u8>) -> Option<TagSymbol<'val>> {
    let size = decode_size(reader)?;
    let (size, is_bytes) = (size >> 1, size & 1 != 0);
    if is_bytes {
        Some(TagSymbol::Bytes(reader.next_n(u64_to_usize(size)?)?))
    }
    else {
        unimplemented!()
    }
}


pub fn decode_var_bytes<'val>(reader: &mut Reader<'val, u8>) -> Option<&'val [u8]> {
    let size = decode_size_as_usize(reader)?;
    reader.next_n(size)
}

// sequence?
pub fn decode_list(buffer: &[u8]) -> Option<(usize, Reader<u8>)> {
    let mut reader = Reader::new(buffer);
    let count =
        if reader.has_some() { decode_size_as_usize(&mut reader)? }
        else                 { 0 };
    Some((count, reader))
}



#[allow(dead_code)]
pub struct Value<'val> {
    pub ty: WireType,
    pub has_kind: bool,
    pub has_tags: bool,

    pub kind: &'val [u8],
    pub tags: &'val [u8],
    pub payload: Payload<'val>,
}

pub fn decode_value<'rdr>(reader: &mut Reader<'rdr, u8>) -> Option<Value<'rdr>> {
    let header = reader.next_u8_le()?;

    let ty: WireType = {
        let ty = header & WIRE_TYPE_MASK;
        if !(ty >= WIRE_TYPE_MIN && ty <= WIRE_TYPE_MAX) {
            return None;
        }
        unsafe { std::mem::transmute(ty) }
    };

    let has_kind = header & WIRE_FLAG_KIND != 0;
    let has_tags = header & WIRE_FLAG_TAGS != 0;

    let mut result = Value {
        ty, has_kind, has_tags,
        kind: &reader.buffer[0..0],
        tags:        &reader.buffer[0..0],
        payload:     Payload::Null,
    };

    if has_kind {
        unimplemented!();
    }

    if has_tags {
        let size = decode_size_as_usize(reader)?;
        result.tags = reader.next_n(size)?;
    }

    result.payload = decode_payload(ty, reader)?;

    Some(result)
}

impl<'val> Value<'val> {
    pub fn tags(&self) -> Option<TagDecoder> {
        TagDecoder::new(self.tags)
    }
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
    String    (&'val str ),
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
        Nat    => { Payload::Nat(decode_var_bytes(reader)?) },
        Int    => { Payload::Int(decode_var_bytes(reader)?) },
        Bytes  => { Payload::Bytes(decode_var_bytes(reader)?) },
        Symbol => { Payload::Symbol(decode_var_bytes(reader)?) },
        List   => { Payload::List(decode_var_bytes(reader)?) },
        String => {
            let bytes = decode_var_bytes(reader)?;
            Payload::String(std::str::from_utf8(bytes).ok()?)
        },
    })
}



pub struct TagDecoder<'val> {
    remaining: usize,
    reader: Reader<'val, u8>,
    error: bool,
}

impl<'val> TagDecoder<'val> {
    pub fn new(tags: &'val [u8]) -> Option<TagDecoder> {
        let (remaining, reader) = decode_list(tags)?;
        if reader.remaining() < 2*remaining {
            return None;
        }
        Some(TagDecoder { remaining, reader, error: false })
    }

    pub fn check_error(self) -> Result<(), ()> {
        if self.error || self.remaining > 0 || self.reader.has_some() {
            return Err(());
        }
        Ok(())
    }

    pub fn remaining(&self) -> usize { self.remaining }
}

impl<'val> Iterator for TagDecoder<'val> {
    type Item = (TagSymbol<'val>, Value<'val>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining > 0 {
            let symbol = match decode_tag_symbol(&mut self.reader) {
                Some(symbol) => symbol,
                None => { self.error = true; return None },
            };

            let value = match decode_value(&mut self.reader) {
                Some(value) => value,
                None => { self.error = true; return None },
            };

            self.remaining -= 1;
            return Some((symbol, value))
        }
        None
    }
}



pub struct ListDecoder<'val> {
    remaining: usize,
    reader: Reader<'val, u8>,
    error: bool,
}

impl<'val> ListDecoder<'val> {
    pub fn new(payload: &'val [u8]) -> Option<ListDecoder> {
        let (remaining, reader) = decode_list(payload)?;
        if reader.remaining() < remaining {
            return None;
        }
        Some(ListDecoder { remaining, reader, error: false })
    }

    pub fn check_error(self) -> Result<(), ()> {
        if self.error || self.remaining > 0 || self.reader.has_some() {
            return Err(());
        }
        Ok(())
    }

    pub fn remaining(&self) -> usize { self.remaining }
}

impl<'val> Iterator for ListDecoder<'val> {
    type Item = Value<'val>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining > 0 {
            let value = match decode_value(&mut self.reader) {
                Some(value) => value,
                None => { self.error = true; return None },
            };

            self.remaining -= 1;
            return Some(value)
        }
        None
    }
}

