pub mod wire_type;
pub mod utils;
pub mod encoder;
pub mod decoder;

pub use wire_type::*;
pub use slice_reader::Reader;



pub fn validate(buffer: &[u8]) -> Result<(), ()> {
    let mut reader = slice_reader::Reader::new(buffer);
    _validate(&decoder::decode_value(&mut reader).ok_or(())?)?;
    if reader.has_some() {
        return Err(())
    }
    Ok(())
}

pub fn _validate(value: &decoder::Value) -> Result<(), ()> {
    if value.header.has_tags {
        let mut tags = value.tags().ok_or(())?;
        for (_symbol, value) in &mut tags {
            _validate(&value)?;
        }
        tags.check_error()?;
    }

    use decoder::Payload::*;
    match value.payload {
        String (value) => {
            std::str::from_utf8(value).ok().ok_or(())?;
        },

        List (value) => {
            let mut payload = decoder::ListDecoder::new(value).ok_or(())?;
            for value in &mut payload {
                _validate(&value)?;
            }
            payload.check_error()?;
        },

        _ => (),
    }

    Ok(())
}

