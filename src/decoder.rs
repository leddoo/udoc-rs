use slice_reader::Reader;
use crate::common::*;


#[derive(Debug, Clone)]
pub enum Error {
    InputExhausted,
    TrailingData,
    SizeTooLarge,
    InvalidWireType,

    StringInvalidUtf8,

    TagsInvalidLength,
    TagsInputExhausted,
    TagsTrailingData,

    ListInvalidLength,
    ListInputExhausted,
    ListTrailingData,
}

pub type Result<T> = std::result::Result<T, Error>;


pub fn u64_to_usize(value: u64) -> Result<usize> {
    value.try_into().ok().ok_or(Error::SizeTooLarge)
}

pub fn decode_size(reader: &mut Reader<u8>) -> Result<u64> {
    crate::common::decode_size(reader).ok_or(Error::InputExhausted)
}

pub fn decode_size_as_usize(reader: &mut Reader<u8>) -> Result<usize> {
    u64_to_usize(decode_size(reader)?)
}


pub enum TagSymbol<'val> {
    Bytes (&'val [u8]),
}

pub fn decode_tag_symbol<'val>(reader: &mut Reader<'val, u8>) -> Result<TagSymbol<'val>> {
    let size = decode_size(reader)?;
    let (size, is_bytes) = (size >> 1, size & 1 != 0);
    if is_bytes {
        Ok(TagSymbol::Bytes(reader.next_n(u64_to_usize(size)?).ok_or(Error::InputExhausted)?))
    }
    else {
        unimplemented!()
    }
}


pub fn decode_var_bytes<'val>(reader: &mut Reader<'val, u8>) -> Result<&'val [u8]> {
    let size = decode_size_as_usize(reader)?;
    reader.next_n(size).ok_or(Error::InputExhausted)
}

pub fn decode_list(buffer: &[u8]) -> Result<(usize, Reader<u8>)> {
    let mut reader = Reader::new(buffer);
    let count =
        if reader.has_some() { decode_size_as_usize(&mut reader)? }
        else                 { 0 };
    Ok((count, reader))
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

pub fn decode_value<'rdr>(reader: &mut Reader<'rdr, u8>) -> Result<Value<'rdr>> {
    let header = reader.next_u8_le().ok_or(Error::InputExhausted)?;

    let ty: WireType = {
        let ty = header & WIRE_TYPE_MASK;
        if !(ty >= WIRE_TYPE_MIN && ty <= WIRE_TYPE_MAX) {
            return Err(Error::InvalidWireType);
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
        result.tags = reader.next_n(size).ok_or(Error::InputExhausted)?;
    }

    result.payload = decode_payload(ty, reader)?;

    Ok(result)
}

impl<'val> Value<'val> {
    pub fn tags(&self) -> Result<TagDecoder> {
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

pub fn decode_payload<'val>(ty: WireType, reader: &mut Reader<'val, u8>) -> Result<Payload<'val>> {
    use WireType::*;
    Ok(match ty {
        Null      => { Payload::Null },
        BoolFalse => { Payload::Bool(false) },
        BoolTrue  => { Payload::Bool(true) },
        Nat8      => { Payload::Nat8(reader.next_u8_le().ok_or(Error::InputExhausted)?) },
        Nat16     => { Payload::Nat16(reader.next_u16_le().ok_or(Error::InputExhausted)?) },
        Nat32     => { Payload::Nat32(reader.next_u32_le().ok_or(Error::InputExhausted)?) },
        Nat64     => { Payload::Nat64(reader.next_u64_le().ok_or(Error::InputExhausted)?) },
        Int8      => { Payload::Int8(reader.next_i8_le().ok_or(Error::InputExhausted)?) },
        Int16     => { Payload::Int16(reader.next_i16_le().ok_or(Error::InputExhausted)?) },
        Int32     => { Payload::Int32(reader.next_i32_le().ok_or(Error::InputExhausted)?) },
        Int64     => { Payload::Int64(reader.next_i64_le().ok_or(Error::InputExhausted)?) },
        Float32   => { Payload::Float32(reader.next_f32_le().ok_or(Error::InputExhausted)?) },
        Float64   => { Payload::Float64(reader.next_f64_le().ok_or(Error::InputExhausted)?) },
        Decimal32 => { Payload::Decimal32(reader.next_bytes_le::<4>().ok_or(Error::InputExhausted)?) },
        Decimal64 => { Payload::Decimal64(reader.next_bytes_le::<8>().ok_or(Error::InputExhausted)?) },
        Nat    => { Payload::Nat(decode_var_bytes(reader)?) },
        Int    => { Payload::Int(decode_var_bytes(reader)?) },
        Bytes  => { Payload::Bytes(decode_var_bytes(reader)?) },
        Symbol => { Payload::Symbol(decode_var_bytes(reader)?) },
        List   => { Payload::List(decode_var_bytes(reader)?) },
        String => {
            let bytes = decode_var_bytes(reader)?;
            Payload::String(std::str::from_utf8(bytes).ok().ok_or(Error::StringInvalidUtf8)?)
        },
    })
}



pub struct TagDecoder<'val> {
    remaining: usize,
    reader: Reader<'val, u8>,
    error: Option<Error>,
}

impl<'val> TagDecoder<'val> {
    pub fn new(tags: &'val [u8]) -> Result<TagDecoder> {
        let (remaining, reader) = decode_list(tags)?;
        if reader.remaining() < 2*remaining {
            return Err(Error::TagsInvalidLength);
        }
        Ok(TagDecoder { remaining, reader, error: None })
    }

    pub fn check_error(self) -> Result<()> {
        if let Some(error) = self.error {
            return Err(error);
        }
        if self.remaining > 0 {
            return Err(Error::TagsInputExhausted);
        }
        if self.reader.has_some() {
            return Err(Error::TagsTrailingData);
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
                Ok(symbol) => symbol,
                Err(error) => { self.error = Some(error); return None },
            };

            let value = match decode_value(&mut self.reader) {
                Ok(value) => value,
                Err(error) => { self.error = Some(error); return None },
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
    error: Option<Error>,
}

impl<'val> ListDecoder<'val> {
    pub fn new(payload: &'val [u8]) -> Result<ListDecoder> {
        let (remaining, reader) = decode_list(payload)?;
        if reader.remaining() < remaining {
            return Err(Error::ListInvalidLength);
        }
        Ok(ListDecoder { remaining, reader, error: None })
    }

    pub fn check_error(self) -> Result<()> {
        if let Some(error) = self.error {
            return Err(error);
        }
        if self.remaining > 0 {
            return Err(Error::ListInputExhausted);
        }
        if self.reader.has_some() {
            return Err(Error::ListTrailingData);
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
                Ok(value) => value,
                Err(error) => { self.error = Some(error); return None },
            };

            self.remaining -= 1;
            return Some(value)
        }
        None
    }
}

