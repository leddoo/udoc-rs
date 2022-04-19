use byte_order::ByteOrder;
use slice_reader::Reader;


pub fn encode_size<B: ByteOrder>(value: u64) -> ([u8; 8], usize) {
    let bits = 64 - value.leading_zeros();

    let value = value << 2;
    let (value, length) =
        if      bits <=  8 - 2 { (value | 0b00, 1) }
        else if bits <= 16 - 2 { (value | 0b01, 2) }
        else if bits <= 32 - 2 { (value | 0b10, 4) }
        else if bits <= 64 - 2 { (value | 0b11, 8) }
        else { unreachable!() };

    (B::write_u64(value), length)
}

pub fn decode_size<B: ByteOrder>(reader: &mut Reader<u8>) -> Option<u64> {
    let first = *reader.peek()?;
    let value = match first & 0b11 {
        0b00 => reader.next_u8::<B>()? as u64,
        0b01 => reader.next_u16::<B>()? as u64,
        0b10 => reader.next_u32::<B>()? as u64,
        0b11 => reader.next_u64::<B>()?,
        _    => unreachable!()
    };
    Some(value >> 2)
}

pub fn peek_decode_size<B: ByteOrder>(reader: &Reader<u8>) -> Option<(u64, usize)> {
    let mut reader = reader.clone();
    let old_cursor = reader.cursor;
    let size = decode_size::<B>(&mut reader)?;
    Some((size, reader.cursor - old_cursor))
}


pub fn u64_to_usize(value: u64) -> Option<usize> {
    value.try_into().ok()
}

pub fn decode_size_as_usize<B: ByteOrder>(reader: &mut Reader<u8>) -> Option<usize> {
    u64_to_usize(decode_size::<B>(reader)?)
}

