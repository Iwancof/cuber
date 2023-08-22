use super::{IResult, TIResult};

use byteorder::ReadBytesExt;

const VARINT_SEGMENT_BITS: u8 = 0x7f;
const VARINT_CONTINUE_BIT: u8 = 0x80;

pub async fn async_read_var_int<T: tokio::io::AsyncReadExt + std::marker::Unpin>(
    d: &mut T,
) -> TIResult<(usize, i32)> {
    let mut value = 0;
    let mut position = 0;
    let mut read = 0;

    loop {
        let current_byte = d.read_u8().await?;
        read += 1;

        let segment = current_byte & VARINT_SEGMENT_BITS;
        value |= (segment as i32) << position;

        if current_byte & VARINT_CONTINUE_BIT == 0 {
            break;
        }

        position += 7;
        if position >= 32 {
            return Err(tokio::io::Error::new(
                tokio::io::ErrorKind::InvalidData,
                "VarInt is too big",
            ));
        }
    }

    Ok((read, value))
}

pub fn read_var_int<T: ReadBytesExt>(d: &mut T) -> IResult<(usize, i32)> {
    let mut value = 0;
    let mut position = 0;
    let mut read = 0;

    loop {
        let current_byte = d.read_u8()?;
        read += 1;

        let segment = current_byte & VARINT_SEGMENT_BITS;
        value |= (segment as i32) << position;

        if current_byte & VARINT_CONTINUE_BIT == 0 {
            break;
        }

        position += 7;
        if position >= 32 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "VarInt is too big",
            ));
        }
    }

    Ok((read, value))
}

pub fn build_var_int(value: i32) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();

    let mut remaining_value = value as u32;

    while remaining_value >= 0x80 {
        result.push((remaining_value & 0x7F | 0x80) as u8);
        remaining_value >>= 7;
    }

    result.push(remaining_value as u8);
    // result.reverse();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_var_int_zero() {
        let value = 0;
        let result = build_var_int(value);
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn test_build_var_int_positive_value() {
        let value = 150;
        let result = build_var_int(value);
        assert_eq!(result, vec![0x96, 0x01]);
    }
}
