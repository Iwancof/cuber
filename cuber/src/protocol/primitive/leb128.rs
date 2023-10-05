use std::marker::Unpin;
use tokio::io::AsyncReadExt;

use byteorder::ReadBytesExt;

const VARINT_SEGMENT_BITS: u8 = 0x7f;
const VARINT_CONTINUE_BIT: u8 = 0x80;

use anyhow::{Context as _, Result, ensure};

pub async fn async_read_var_int<T: AsyncReadExt + Unpin>(d: &mut T) -> Result<(usize, i32)> {
    let mut value = 0;
    let mut position = 0;
    let mut read = 0;

    loop {
        let current_byte = d.read_u8().await.context("Failed to read byte(Async)")?;
        read += 1;

        let segment = current_byte & VARINT_SEGMENT_BITS;
        value |= (segment as i32) << position;

        if current_byte & VARINT_CONTINUE_BIT == 0 {
            break;
        }

        position += 7;
        ensure!(position < 32, "VarInt is too big");
    }

    Ok((read, value))
}

pub fn read_var_int<T: ReadBytesExt>(d: &mut T) -> Result<(usize, i32)> {
    let mut value = 0;
    let mut position = 0;
    let mut read = 0;

    loop {
        let current_byte = d.read_u8().context("Failed to read byte")?;
        read += 1;

        let segment = current_byte & VARINT_SEGMENT_BITS;
        value |= (segment as i32) << position;

        if current_byte & VARINT_CONTINUE_BIT == 0 {
            break;
        }

        position += 7;
        ensure!(position < 32, "VarInt is too big");
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

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    macro_rules! make_build_test_case {
        ($name: ident, $bytes: expr, $value: expr) => {
            #[test]
            fn $name() {
                let value = $value;
                let result = build_var_int(value);
                assert_eq!(result, $bytes);
            }
        };
    }

    macro_rules! make_read_test_case {
        ($name: ident, $bytes: expr, $value: expr) => {
            #[test]
            fn $name() {
                use std::io::Cursor;

                let bytes = $bytes;
                let length = bytes.len();
                let mut bytes = Cursor::new(bytes);
                let (len, v) = read_var_int(&mut bytes).unwrap();
                assert_eq!(v, $value);
                assert_eq!(length, len);
            }
        };
    }
    make_build_test_case!(test_build_var_int_zero, vec![0], 0);
    make_build_test_case!(test_build_var_int_one, vec![1], 1);
    make_build_test_case!(test_build_var_int_two, vec![2], 2);
    make_build_test_case!(test_build_var_int_127, vec![0x7f], 127);
    make_build_test_case!(test_build_var_int_128, vec![0x80, 0x01], 128);
    make_build_test_case!(test_build_var_int_255, vec![0xff, 0x01], 255);
    make_build_test_case!(test_build_var_int_25565, vec![0xdd, 0xc7, 0x01], 25565);
    make_build_test_case!(test_build_var_int_2097151, vec![0xff, 0xff, 0x7f], 2097151);
    make_build_test_case!(
        test_build_var_int_2147483647,
        vec![0xff, 0xff, 0xff, 0xff, 0x07],
        2147483647
    );
    make_build_test_case!(
        test_build_var_int_minus_1,
        vec![0xff, 0xff, 0xff, 0xff, 0x0f],
        -1
    );
    make_build_test_case!(
        test_build_var_int_minus_2147483648,
        vec![0x80, 0x80, 0x80, 0x80, 0x08],
        -2147483648
    );

    make_read_test_case!(test_read_var_int_zero, vec![0], 0);
    make_read_test_case!(test_read_var_int_one, vec![1], 1);
    make_read_test_case!(test_read_var_int_two, vec![2], 2);
    make_read_test_case!(test_read_var_int_127, vec![0x7f], 127);
    make_read_test_case!(test_read_var_int_128, vec![0x80, 0x01], 128);
    make_read_test_case!(test_read_var_int_255, vec![0xff, 0x01], 255);
    make_read_test_case!(test_read_var_int_25565, vec![0xdd, 0xc7, 0x01], 25565);
    make_read_test_case!(test_read_var_int_2097151, vec![0xff, 0xff, 0x7f], 2097151);
    make_read_test_case!(
        test_read_var_int_2147483647,
        vec![0xff, 0xff, 0xff, 0xff, 0x07],
        2147483647
    );
    make_read_test_case!(
        test_read_var_int_minus_1,
        vec![0xff, 0xff, 0xff, 0xff, 0x0f],
        -1
    );
    make_read_test_case!(
        test_read_var_int_minus_2147483648,
        vec![0x80, 0x80, 0x80, 0x80, 0x08],
        -2147483648
    );
}
