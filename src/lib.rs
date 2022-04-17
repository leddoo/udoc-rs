pub mod wire_type;
pub mod utils;
pub mod encoder;
pub mod decoder;


pub use wire_type::*;


use slice_reader::{Reader};
use encoder::{Encoder};
use decoder::{decode_value, ListDecoder, TagSymbol};



fn validate(buffer: &[u8]) -> decoder::Result<()> {
    let mut reader = Reader::new(buffer);
    _validate(&decode_value(&mut reader)?)?;
    if reader.has_some() {
        return Err(decoder::Error::TrailingData);
    }
    Ok(())
}

fn _validate(value: &decoder::Value) -> decoder::Result<()> {
    if value.has_tags {
        let mut tags = value.tags()?;
        for (_symbol, value) in &mut tags {
            _validate(&value)?;
        }
        tags.check_error()?;
    }

    use decoder::Payload::*;
    match value.payload {
        List (value) => {
            let mut payload = ListDecoder::new(value)?;
            for value in &mut payload {
                _validate(&value)?;
            }
            payload.check_error()?;
        },

        _ => (),
    }

    Ok(())
}



use serde_json::{Value};

fn encode_json(value: &Value) -> Vec<u8> {
    let mut encoder = Encoder::default();
    _encode_json(&mut encoder, value);
    encoder.build().unwrap()
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



// this should technically use its own error.

fn decode_json(buffer: &[u8]) -> decoder::Result<Value> {
    let mut reader = Reader::new(buffer);
    let value = _decode_json(&decode_value(&mut reader)?)?;
    if reader.has_some() {
        return Err(decoder::Error::TrailingData);
    }
    Ok(value)
}

fn _decode_json(value: &decoder::Value) -> decoder::Result<Value> {
    use decoder::Payload::*;
    let result = match value.payload {
        Null => {
            if value.has_tags {
                let mut map = serde_json::Map::new();

                let mut tags = value.tags()?;
                for (symbol, value) in &mut tags {
                    let symbol = match symbol { TagSymbol::Bytes (symbol) => symbol, };
                    let symbol = std::string::String::from_utf8(symbol.into()).ok().ok_or(decoder::Error::StringInvalidUtf8)?;
                    let value = _decode_json(&value)?;
                    map.insert(symbol, value);
                }
                tags.check_error()?;

                Value::Object(map)
            }
            else {
                Value::Null
            }
        },

        Bool (value) => { Value::Bool(value) },

        Float64 (value) => {
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

        String (value) => {
            Value::String(value.into())
        },

        List (value) => {
            let mut payload = ListDecoder::new(value)?;

            let mut values = vec![];
            values.reserve(payload.remaining());
            for value in &mut payload {
                values.push(_decode_json(&value)?);
            }
            payload.check_error()?;

            Value::Array(values)
        },

        _ => { return Err(decoder::Error::InvalidWireType); },
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


pub fn main() {
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


    if 1 == 1 {
        let v: Value = serde_json::from_slice(sleep).unwrap();
        let length = encode_json(&v).len();
        bench("sleep encode compressed", length, || {
            encode_json(&v);
        });
    }

    if 1 == 1 {
        let v: Value = serde_json::from_slice(sleep).unwrap();
        let length = {
            let mut encoder = Encoder::new(4, false);
            _encode_json(&mut encoder, &v);
            encoder.build().unwrap().len()
        };
        bench("sleep encode uncompressed", length, || {
            let mut encoder = Encoder::new(4, false);
            _encode_json(&mut encoder, &v);
            encoder.build().unwrap();
        });
    }


    if 1 == 1 {
        let v: Value = serde_json::from_slice(sleep).unwrap();
        let udoc = encode_json(&v);
        let length = udoc.len();
        bench("sleep validate udoc", length, || {
            validate(&udoc).unwrap();
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
        bench("twitter validate udoc", length, || {
            validate(&udoc).unwrap();
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
        bench("canada validate udoc", length, || {
            validate(&udoc).unwrap();
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

