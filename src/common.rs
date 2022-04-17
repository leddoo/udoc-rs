use slice_reader::Reader;

#[allow(dead_code)]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireType {
    Null        =  1,
    BoolFalse   =  2,
    BoolTrue    =  3,
    Nat8        =  4,
    Nat16       =  5,
    Nat32       =  6,
    Nat64       =  7,
    Int8        =  8,
    Int16       =  9,
    Int32       = 10,
    Int64       = 11,
    Float32     = 12,
    Float64     = 13,
    Decimal32   = 14,
    Decimal64   = 15,
    Nat         = 16,
    Int         = 17,
    Bytes       = 18,
    String      = 19,
    Symbol      = 20,
    List        = 21,
}

pub const WIRE_TYPE_MIN:  u8 =  1;
pub const WIRE_TYPE_MAX:  u8 = 21;
pub const WIRE_TYPE_MASK: u8 = 32 - 1;

pub const WIRE_FLAG_KIND: u8 = 0x40;
pub const WIRE_FLAG_TAGS: u8 = 0x80;


pub fn encode_size(value: u64) -> ([u8; 8], usize) {
    let bits = 64 - value.leading_zeros();

    let value = value << 2;
    let (value, length) =
        if      bits <=  8 - 2 { (value | 0b00, 1) }
        else if bits <= 16 - 2 { (value | 0b01, 2) }
        else if bits <= 32 - 2 { (value | 0b10, 4) }
        else if bits <= 64 - 2 { (value | 0b11, 8) }
        else { unreachable!() };

    (value.to_le_bytes(), length)
}

pub fn decode_size(reader: &mut Reader<u8>) -> Option<u64> {
    let first = *reader.peek()?;
    let value = match first & 0b11 {
        0b00 => reader.next_u8_le()? as u64,
        0b01 => reader.next_u16_le()? as u64,
        0b10 => reader.next_u32_le()? as u64,
        0b11 => reader.next_u64_le()?,
        _    => unreachable!()
    };
    Some(value >> 2)
}

pub fn peek_decode_size(reader: &Reader<u8>) -> Option<(u64, usize)> {
    let mut reader = reader.clone();
    let old_cursor = reader.cursor;
    let size = decode_size(&mut reader)?;
    Some((size, reader.cursor - old_cursor))
}

