
#[derive(Clone)]
struct Reader<'buf, T> {
    pub buffer: &'buf [T],
    pub cursor: usize,
}

impl<'rdr, T> Reader<'rdr, T> {
    pub fn new(buffer: &'rdr [T]) -> Reader<'rdr, T> {
        Reader { buffer: buffer, cursor: 0 }
    }

    pub fn peek(&self, offset: usize) -> Option<&'rdr T> {
        self.buffer.get(self.cursor + offset)
    }

    pub fn empty(&self) -> bool {
        !self.has_some()
    }

    pub fn has_some(&self) -> bool {
        self.cursor < self.buffer.len()
    }

    pub fn next(&mut self) -> Option<&'rdr T> {
        self.buffer.get(self.cursor).map(|result| {
            self.cursor += 1;
            result
        })
    }

    pub fn has_n(&self, n: usize) -> bool {
        self.cursor + n <= self.buffer.len()
    }

    pub fn peek_next_n(&self, n: usize) -> Option<&'rdr [T]> {
        if self.has_n(n) {
            return Some(&self.buffer[self.cursor .. self.cursor + n]);
        }
        None
    }

    pub fn next_n(&mut self, n: usize) -> Option<&'rdr [T]> {
        self.peek_next_n(n).map(|result| {
            self.cursor += n;
            result
        })
    }

    pub fn rest(&self) -> &[T] {
        &self.buffer[self.cursor..]
    }
}

impl<'rdr> Reader<'rdr, u8> {
    pub fn next_u8(&mut self) -> Option<u8> {
        self.next().map(|result| *result)
    }

    pub fn next_u16_le(&mut self) -> Option<u16> {
        if self.has_n(2) {
            let bytes = unsafe {[
                *self.buffer.get_unchecked(self.cursor + 0),
                *self.buffer.get_unchecked(self.cursor + 1),
            ]};
            self.cursor += 2;
            return Some(u16::from_le_bytes(bytes))
        }
        None
    }

    pub fn next_u32_le(&mut self) -> Option<u32> {
        if self.has_n(4) {
            let bytes = unsafe {[
                *self.buffer.get_unchecked(self.cursor + 0),
                *self.buffer.get_unchecked(self.cursor + 1),
                *self.buffer.get_unchecked(self.cursor + 2),
                *self.buffer.get_unchecked(self.cursor + 3),
            ]};
            self.cursor += 4;
            return Some(u32::from_le_bytes(bytes))
        }
        None
    }

    pub fn next_u64_le(&mut self) -> Option<u64> {
        if self.has_n(8) {
            let bytes = unsafe {[
                *self.buffer.get_unchecked(self.cursor + 0),
                *self.buffer.get_unchecked(self.cursor + 1),
                *self.buffer.get_unchecked(self.cursor + 2),
                *self.buffer.get_unchecked(self.cursor + 3),
                *self.buffer.get_unchecked(self.cursor + 4),
                *self.buffer.get_unchecked(self.cursor + 5),
                *self.buffer.get_unchecked(self.cursor + 6),
                *self.buffer.get_unchecked(self.cursor + 7),
            ]};
            self.cursor += 8;
            return Some(u64::from_le_bytes(bytes))
        }
        None
    }
}



#[allow(dead_code)]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WireType {
    Null        =  1,
    BoolFalse   =  2,
    BoolTrue    =  3,
    Nat         =  4,
    Nat8        =  5,
    Nat16       =  6,
    Nat32       =  7,
    Nat64       =  8,
    Int         =  9,
    Int8        = 10,
    Int16       = 11,
    Int32       = 12,
    Int64       = 13,
    Float32     = 14,
    Float64     = 15,
    Decimal32   = 16,
    Decimal64   = 17,
    Bytes       = 18,
    String      = 19,
    Symbol      = 20,
    List        = 21,
}

const WIRE_TYPE_MIN:  u8 =  1;
const WIRE_TYPE_MAX:  u8 = 21;
const WIRE_TYPE_MASK: u8 = 32 - 1;

const WIRE_FLAG_SCHEMA_TYPE: u8 = 0x40;
const WIRE_FLAG_TAGS:        u8 = 0x80;



// value: Nat62
fn encode_size(value: u64) -> ([u8; 8], usize) {
    let bits = 64 - value.leading_zeros();

    let value = value << 2;
    let (value, length) =
        if      bits <= 6      { (value | 0b00, 1) }
        else if bits <= 6 +  8 { (value | 0b01, 2) }
        else if bits <= 6 + 24 { (value | 0b10, 4) }
        else                   { (value | 0b11, 8) };

    (value.to_le_bytes(), length)
}

#[inline(always)]
fn decode_size(reader: &mut Reader<u8>) -> Result<u64, ()> {
    let first = reader.peek(0).ok_or(())?;
    let value = match first & 0b11 {
        0b00 => reader.next_u8().ok_or(())? as u64,
        0b01 => reader.next_u16_le().ok_or(())? as u64,
        0b10 => reader.next_u32_le().ok_or(())? as u64,
        0b11 => reader.next_u64_le().ok_or(())?,
        _    => unreachable!()
    };
    Ok(value >> 2)
}

fn peek_decode_size(reader: &Reader<u8>) -> Result<(u64, usize), ()> {
    let mut reader = reader.clone();
    let old_cursor = reader.cursor;
    let size = decode_size(&mut reader)?;
    Ok((size, reader.cursor - old_cursor))
}


// value: Nat14
fn encode_symbol_length(value: u64) -> ([u8; 8], usize) {
    let bits = 64 - value.leading_zeros();

    let value = value << 2;
    let (value, length) =
        if bits < 6 { (value | 0b00, 1) }
        else        { (value | 0b01, 2) };

    (value.to_le_bytes(), length)
}

#[inline(always)]
fn decode_symbol_length(reader: &mut Reader<u8>) -> Result<u64, ()> {
    let first = *reader.peek(0).ok_or(())?;

    let is_inline = first & 0b10 == 0;
    if is_inline {
        let value = match first & 0b01 {
            0b00 => reader.next_u8().ok_or(())?     as u64,
            0b01 => reader.next_u16_le().ok_or(())? as u64,
            _ => unreachable!(),
        };
        Ok(value >> 2)
    }
    else {
        unimplemented!()
    }
}



struct Encoder {
    buffer: Vec<u8>,
    sizers: Vec<Sizer>,

    size_offsets: Vec<u8>, // relative, encoded with udoc size encoding.
    last_size_offset: usize,
}

struct Sizer {
    offset: usize,
    size:    usize,
}

impl Encoder {
    pub fn new() -> Encoder {
        Encoder {
            buffer: vec![],
            sizers: vec![ Sizer { offset: 0, size: 0 } ],

            size_offsets: vec![],
            last_size_offset: 0,
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
        assert!(symbol.len() < 1 << 14);

        let (bytes, length) = encode_symbol_length(symbol.len() as u64);
        self.append(&bytes[..length]);
        self.append(symbol);
    }


    pub fn begin_size(&mut self) {
        let offset = self.buffer.len();
        self.buffer.extend((0..8).map(|_| 0));
        self.sizers.push(Sizer { offset: offset, size: 0 });

        let delta = (offset - self.last_size_offset) as u64;
        let (delta, length) = encode_size(delta);
        self.size_offsets.extend(&delta[..length]);
        self.last_size_offset = offset;
    }

    pub fn end_size(&mut self) {
        assert!(self.sizers.len() > 1);
        let sizer = self.sizers.pop().unwrap();

        let (size, length) = encode_size(sizer.size as u64);
        let buffer = &mut self.buffer[sizer.offset .. sizer.offset + 8];
        buffer.copy_from_slice(&size);

        self.commit_size(length + sizer.size);
    }

    pub fn compress(&self) -> Vec<u8> {
        assert!(self.sizers.len() == 1);

        let size = self.sizers.last().unwrap().size;
        let mut string = Vec::with_capacity(size);

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
                    // sizes in the buffer. the previous size (8 bytes in the
                    // buffer) is already consumed.
                    next_size -= 8;
                }

                string.extend(buffer.next_n(next_size).unwrap());

                let (_size, length) = peek_decode_size(&buffer).unwrap();
                string.extend(buffer.peek_next_n(length).unwrap());
                buffer.next_n(8).unwrap();
            }
            else {
                string.extend(buffer.rest());
                break;
            }
        }
        assert_eq!(string.len(), size);

        string
    }
}


#[allow(dead_code)]
struct DecodedValue<'val> {
    ty: WireType,
    has_schema_type: bool,
    has_tags: bool,

    schema_type: &'val [u8],
    tags: &'val [u8],
    payload: &'val [u8],
}

fn decode<'rdr>(reader: &mut Reader<'rdr, u8>) -> Result<DecodedValue<'rdr>, ()> {
    let header = reader.next_u8().ok_or(())?;

    let ty: WireType = {
        let ty = header & WIRE_TYPE_MASK;
        if !(ty >= WIRE_TYPE_MIN && ty <= WIRE_TYPE_MAX) {
            return Err(());
        }
        unsafe { std::mem::transmute(ty) }
    };

    let has_schema_type = header & WIRE_FLAG_SCHEMA_TYPE != 0;
    let has_tags        = header & WIRE_FLAG_TAGS != 0;

    let mut result = DecodedValue {
        ty, has_schema_type, has_tags,
        schema_type: &reader.buffer[0..0],
        tags:        &reader.buffer[0..0],
        payload:     &reader.buffer[0..0],
    };

    if has_schema_type {
        unimplemented!();
    }

    if has_tags {
        let size = decode_size(reader)? as usize;
        result.tags = reader.next_n(size).ok_or(())?;
    }

    use WireType::*;
    result.payload = match ty {
        Null | BoolFalse | BoolTrue => {
            &reader.buffer[0..0]
        },
        Nat8 | Int8 => {
            reader.next_n(1).ok_or(())?
        },
        Nat16 | Int16 => {
            reader.next_n(2).ok_or(())?
        },
        Nat32 | Int32 | Float32 | Decimal32 => {
            reader.next_n(4).ok_or(())?
        },
        Nat64 | Int64 | Float64 | Decimal64 => {
            reader.next_n(8).ok_or(())?
        },
        Nat | Int | Bytes | String | List => {
            let size = decode_size(reader)? as usize;
            reader.next_n(size).ok_or(())?
        },
        Symbol => {
            unimplemented!()
        },
    };

    Ok(result)
}



use serde_json::{Value};

fn encode_json(value: &Value) -> Vec<u8> {
    let mut encoder = Encoder::new();
    _encode_json(&mut encoder, value);
    encoder.compress()
}

fn _encode_json(encoder: &mut Encoder, value: &Value) {
    match value {
        Value::Null => {
            encoder.append_byte(WireType::Null as u8);
        },
        Value::Bool (value) => {
            if *value {
                encoder.append_byte(WireType::BoolTrue as u8);
            }
            else {
                encoder.append_byte(WireType::BoolFalse as u8);
            }
        },
        Value::Number (value) => {
            encoder.append_byte(WireType::Float64 as u8);
            encoder.append(&value.as_f64().unwrap().to_le_bytes());
        },
        Value::String (value) => {
            encoder.append_byte(WireType::String as u8);
            encoder.append_size(value.len() as u64);
            encoder.append(value.as_bytes());
        },
        Value::Array (value) => {
            encoder.append_byte(WireType::List as u8);

            if value.len() > 0 {
                encoder.begin_size();
                encoder.append_size(value.len() as u64);
                for entry in value {
                    _encode_json(encoder, entry);
                }
                encoder.end_size();
            }
            else {
                encoder.append_byte(0);
            }
        },
        Value::Object (value) => {
            encoder.append_byte(WireType::Null as u8 | WIRE_FLAG_TAGS);

            if value.len() > 0 {
                encoder.begin_size();
                encoder.append_size(value.len() as u64);
                for (k, v) in value {
                    encoder.append_symbol(k.as_ref());
                    _encode_json(encoder, v);
                }
                encoder.end_size();
            }
            else {
                encoder.append_byte(0);
            }
        },
    }
}



fn decode_json(buffer: &[u8]) -> Result<Value, ()> {
    let mut reader = Reader::new(buffer);
    let value = _decode_json(&mut reader)?;
    if reader.has_some() {
        return Err(());
    }
    Ok(value)
}

fn _decode_json(reader: &mut Reader<u8>) -> Result<Value, ()> {
    let value = decode(reader)?;

    use WireType::*;
    let result = match value.ty {
        Null => {
            if value.has_tags {
                let mut map = serde_json::Map::new();

                if value.tags.len() > 0 {
                    let mut tags = Reader::new(&value.tags);

                    let count = decode_size(&mut tags)? as usize;
                    for _ in 0..count {
                        let symbol_size = decode_symbol_length(&mut tags)? as usize;

                        let symbol = Vec::from(tags.next_n(symbol_size).ok_or(())?);
                        let symbol = std::string::String::from_utf8(symbol).ok().ok_or(())?;

                        let value = _decode_json(&mut tags)?;
                        map.insert(symbol, value);
                    }
                }

                Value::Object(map)
            }
            else {
                Value::Null
            }
        },

        BoolFalse => { Value::Bool(false) },
        BoolTrue  => { Value::Bool(true) },

        Float64 => {
            let value = {
                let mut bytes = [0; 8];
                bytes.copy_from_slice(value.payload);
                f64::from_le_bytes(bytes)
            };

            // temp
            let number =
                if value as u64 as f64 == value {
                    if value >= 0.0 {
                        serde_json::Number::from(value as u64)
                    }
                    else {
                        serde_json::Number::from(value as i64)
                    }
                }
                else {
                    serde_json::Number::from_f64(value).unwrap()
                };

            Value::Number(number)
        },

        String => {
            let string = value.payload.to_vec();
            let string = std::string::String::from_utf8(string).ok().ok_or(())?;
            Value::String(string)
        },

        List => {
            let mut payload = Reader::new(&value.payload);

            let mut values = vec![];
            if payload.has_some() {
                let length = decode_size(&mut payload)? as usize;
                values.reserve(length);
                for _ in 0..length {
                    values.push(_decode_json(&mut payload)?);
                }
                assert!(payload.empty());
            }

            Value::Array(values)
        },

        Nat | Nat8 | Nat16 | Nat32 | Nat64 |
        Int | Int8 | Int16 | Int32 | Int64 |
        Float32 |
        Decimal32 | Decimal64 |
        Bytes | Symbol
        => {
            return Err(());
        },
    };

    Ok(result)
}



const BENCH_DURATION: std::time::Duration = std::time::Duration::from_secs(2);
const BENCH_ITERS: usize = 10;

fn bench<F: FnMut()>(name: &str, length: usize, mut f: F) {
    let mut iters = 0;
    let dt;
    let t0 = std::time::Instant::now();
    loop {
        for _ in 0..BENCH_ITERS {
            f();
            iters += 1;
        }

        let elapsed = t0.elapsed();
        if elapsed >= BENCH_DURATION {
            dt = elapsed;
            break;
        }
    }

    let iters_per_sec = iters as f64 / dt.as_secs_f64();
    let mibs = length as f64 * iters_per_sec / (1024.0 * 1024.0);
    println!("{} {:.2}/s {:.2?} {:.2}MiB/s", name, iters_per_sec, dt/iters, mibs);
}

fn main() {
    let example = include_bytes!("example.json");
    let sleep = include_bytes!("sleep.json");
    let twitter = include_bytes!("twitter.json");
    let canada = include_bytes!("canada.json");

    let v: Value = serde_json::from_slice(example).unwrap();
    assert_eq!(v, decode_json(&encode_json(&v)).unwrap());

    let v: Value = serde_json::from_slice(sleep).unwrap();
    assert_eq!(v, decode_json(&encode_json(&v)).unwrap());

    // TODO: broken?
    //let v: Value = serde_json::from_slice(twitter).unwrap();
    //assert_eq!(v, decode_json(&encode_json(&v)).unwrap());

    // TODO: broken?
    //let v: Value = serde_json::from_slice(canada).unwrap();
    //assert_eq!(v, decode_json(&encode_json(&v)).unwrap());


    if 0 == 1 {
        let length = encode_json(&v).len();
        bench("sleep encode compressed", length, || {
            encode_json(&v);
        });
    }

    if 0 == 1 {
        let length = {
            let mut encoder = Encoder::new();
            _encode_json(&mut encoder, &v);
            encoder.buffer.len()
        };
        bench("sleep encode uncompressed", length, || {
            let mut encoder = Encoder::new();
            _encode_json(&mut encoder, &v);
        });
    }


    if 1 == 1 {
        let udoc = encode_json(&v);
        let length = udoc.len();

        bench("sleep decode udoc", length, || {
            decode_json(&udoc).unwrap();
        });
    }

    if 0 == 1 {
        let length = sleep.len();
        bench("sleep decode json", length, || {
            let _: Value = serde_json::from_slice(sleep).unwrap();
        });
    }

    if 0 == 1 {
        let v: Value = serde_json::from_slice(sleep).unwrap();
        let length = sleep.len();
        bench("sleep clone", length, || {
            let _ = v.clone();
        });
    }


    if 1 == 1 {
        let v: Value = serde_json::from_slice(twitter).unwrap();
        let udoc = encode_json(&v);
        let length = udoc.len();
        bench("twitter decode udoc", length, || {
            decode_json(&udoc).unwrap();
        });
    }

    if 0 == 1 {
        let length = twitter.len();
        bench("twitter decode json", length, || {
            let _: Value = serde_json::from_slice(twitter).unwrap();
        });
    }

    if 0 == 1 {
        let v: Value = serde_json::from_slice(twitter).unwrap();
        let length = twitter.len();
        bench("twitter clone", length, || {
            let _ = v.clone();
        });
    }
    

    if 1 == 1 {
        let v: Value = serde_json::from_slice(canada).unwrap();
        let udoc = encode_json(&v);
        let length = udoc.len();
        bench("canada decode udoc", length, || {
            decode_json(&udoc).unwrap();
        });
    }

    if 0 == 1 {
        let length = canada.len();
        bench("canada decode json", length, || {
            let _: Value = serde_json::from_slice(canada).unwrap();
        });
    }

    if 0 == 1 {
        let v: Value = serde_json::from_slice(canada).unwrap();
        let length = canada.len();
        bench("canada clone", length, || {
            let _ = v.clone();
        });
    }
}

