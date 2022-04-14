use crate::reader::Reader;
use crate::common::*;


pub enum TagSymbol<'val> {
    Bytes (&'val [u8]),
}

pub fn decode_tag_symbol<'val>(reader: &mut Reader<'val, u8>) -> Option<TagSymbol<'val>> {
    let size = decode_size(reader)?;
    let (size, is_bytes) = (size >> 1, size & 1 != 0);
    if is_bytes {
        Some(TagSymbol::Bytes(reader.next_n(size.try_into().ok()?)?))
    }
    else {
        unimplemented!()
    }
}


pub fn decode_var_bytes<'val>(reader: &mut Reader<'val, u8>) -> Option<&'val [u8]> {
    let size: usize = decode_size(reader)?.try_into().ok()?;
    reader.next_n(size)
}

pub fn decode_list(buffer: &[u8]) -> Option<(usize, Reader<u8>)> {
    let mut reader = Reader::new(buffer);
    let count =
        if reader.has_some() { decode_size(&mut reader)?.try_into().ok()? }
        else                 { 0 };
    Some((count, reader))
}



#[allow(dead_code)]
pub struct Value<'val> {
    pub ty: WireType,
    pub has_schema_type: bool,
    pub has_tags: bool,

    pub schema_type: &'val [u8],
    pub tags: &'val [u8],
    pub payload: Payload<'val>,
}

pub fn decode_value<'rdr>(reader: &mut Reader<'rdr, u8>) -> Result<Value<'rdr>, ()> {
    let header = reader.next_u8_le().ok_or(())?;

    let ty: WireType = {
        let ty = header & WIRE_TYPE_MASK;
        if !(ty >= WIRE_TYPE_MIN && ty <= WIRE_TYPE_MAX) {
            return Err(());
        }
        unsafe { std::mem::transmute(ty) }
    };

    let has_schema_type = header & WIRE_FLAG_SCHEMA_TYPE != 0;
    let has_tags        = header & WIRE_FLAG_TAGS != 0;

    let mut result = Value {
        ty, has_schema_type, has_tags,
        schema_type: &reader.buffer[0..0],
        tags:        &reader.buffer[0..0],
        payload:     Payload::Null,
    };

    if has_schema_type {
        unimplemented!();
    }

    if has_tags {
        let size: usize = decode_size(reader).ok_or(())?.try_into().ok().ok_or(())?;
        result.tags = reader.next_n(size).ok_or(())?;
    }

    result.payload = decode_payload(ty, reader)?;

    Ok(result)
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

pub fn decode_payload<'val>(ty: WireType, reader: &mut Reader<'val, u8>) -> Result<Payload<'val>, ()> {
    use WireType::*;
    Ok(match ty {
        Null      => { Payload::Null },
        BoolFalse => { Payload::Bool(false) },
        BoolTrue  => { Payload::Bool(true) },
        Nat8      => { Payload::Nat8(reader.next_u8_le().ok_or(())?) },
        Nat16     => { Payload::Nat16(reader.next_u16_le().ok_or(())?) },
        Nat32     => { Payload::Nat32(reader.next_u32_le().ok_or(())?) },
        Nat64     => { Payload::Nat64(reader.next_u64_le().ok_or(())?) },
        Int8      => { Payload::Int8(reader.next_i8_le().ok_or(())?) },
        Int16     => { Payload::Int16(reader.next_i16_le().ok_or(())?) },
        Int32     => { Payload::Int32(reader.next_i32_le().ok_or(())?) },
        Int64     => { Payload::Int64(reader.next_i64_le().ok_or(())?) },
        Float32   => { Payload::Float32(reader.next_f32_le().ok_or(())?) },
        Float64   => { Payload::Float64(reader.next_f64_le().ok_or(())?) },
        Decimal32 => { Payload::Decimal32(reader.next_bytes_le::<4>().ok_or(())?) },
        Decimal64 => { Payload::Decimal64(reader.next_bytes_le::<8>().ok_or(())?) },
        Nat    => { Payload::Nat(decode_var_bytes(reader).ok_or(())?) },
        Int    => { Payload::Int(decode_var_bytes(reader).ok_or(())?) },
        Bytes  => { Payload::Bytes(decode_var_bytes(reader).ok_or(())?) },
        Symbol => { Payload::Symbol(decode_var_bytes(reader).ok_or(())?) },
        List   => { Payload::List(decode_var_bytes(reader).ok_or(())?) },
        String => {
            let bytes = decode_var_bytes(reader).ok_or(())?;
            Payload::String(std::str::from_utf8(bytes).ok().ok_or(())?)
        },
    })
}



pub struct TagDecoder<'val> {
    pub remaining: usize,
    pub reader: Reader<'val, u8>,
}

impl<'val> TagDecoder<'val> {
    pub fn new(tags: &'val [u8]) -> Option<TagDecoder> {
        let (remaining, reader) = decode_list(tags)?;
        if reader.remaining() < 2*remaining {
            return None;
        }
        Some(TagDecoder { remaining, reader })
    }

    pub fn check_error(&self) -> Result<(), ()> {
        if self.remaining > 0 || self.reader.has_some() {
            return Err(());
        }
        Ok(())
    }
}

impl<'val> Iterator for TagDecoder<'val> {
    type Item = (TagSymbol<'val>, Value<'val>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining > 0 {
            let symbol = decode_tag_symbol(&mut self.reader)?;
            let value  = decode_value(&mut self.reader).ok()?;
            self.remaining -= 1;
            return Some((symbol, value))
        }
        None
    }
}



pub struct ListDecoder<'val> {
    pub remaining: usize,
    pub reader: Reader<'val, u8>,
}

impl<'val> ListDecoder<'val> {
    pub fn new(payload: &'val [u8]) -> Option<ListDecoder> {
        let (remaining, reader) = decode_list(payload)?;
        if reader.remaining() < remaining {
            return None;
        }
        Some(ListDecoder { remaining, reader })
    }

    pub fn check_error(&self) -> Result<(), ()> {
        if self.remaining > 0 || self.reader.has_some() {
            return Err(());
        }
        Ok(())
    }
}

impl<'val> Iterator for ListDecoder<'val> {
    type Item = Value<'val>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining > 0 {
            let value = decode_value(&mut self.reader).ok()?;
            self.remaining -= 1;
            return Some(value)
        }
        None
    }
}

