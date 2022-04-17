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
