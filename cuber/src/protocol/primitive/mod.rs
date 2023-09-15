pub mod leb128;

use self::leb128::read_var_int;

use super::CResult;
use super::{Decodable, Encodable};

use deriver::{Decodable, Encodable};

use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

macro_rules! write_primitive {
    ($writer: ident, write_u8, $value: expr) => {{
        let bind = $value;
        $writer
            .write_u8(bind)
            .expect(&format!("could not write data. {}({})", "write_u8", &bind));
        1
    }};
    ($writer: ident, write_i8, $value: expr) => {{
        let bind = $value;
        $writer
            .write_i8(bind)
            .expect(&format!("could not write data. {}({})", "write_i8", &bind));
        1
    }};
    ($writer: ident, $method: ident, $value: expr) => {{
        fn get_size<T>(_: &T) -> usize {
            std::mem::size_of::<T>()
        }

        let bind = $value;
        let len = get_size(&bind);
        $writer.$method::<NetworkEndian>(bind).expect(&format!(
            "could not write data. {}({})",
            stringify!($method),
            &bind
        ));

        len
    }};
}

macro_rules! read_primitive {
    ($reader: ident, read_u8) => {
        $reader.read_u8()?
    };
    ($reader: ident, read_i8) => {
        $reader.read_i8()?
    };
    ($reader: ident, $method: ident) => {
        $reader.$method::<NetworkEndian>()?
    };
}

impl Encodable for bool {
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        write_primitive!(writer, write_u8, if *self { 1 } else { 0 })
    }
}
impl Decodable for bool {
    fn decode<T: Read>(reader: &mut T) -> CResult<Self> {
        Ok(read_primitive!(reader, read_u8) == 1)
    }
}

macro_rules! define_prim {
    ($type: ty, $write_method: ident, $read_method: ident) => {
        impl Encodable for $type {
            fn encode<T: Write>(&self, writer: &mut T) -> usize {
                write_primitive!(writer, $write_method, *self)
            }
        }
        impl Decodable for $type {
            fn decode<T: Read>(reader: &mut T) -> CResult<Self> {
                Ok(read_primitive!(reader, $read_method))
            }
        }
    };
}

define_prim!(i8, write_i8, read_i8);
define_prim!(u8, write_u8, read_u8);
define_prim!(i16, write_i16, read_i16);
define_prim!(u16, write_u16, read_u16);
define_prim!(i32, write_i32, read_i32);
define_prim!(i64, write_i64, read_i64);
define_prim!(f32, write_f32, read_f32);
define_prim!(f64, write_f64, read_f64);

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct VarInt(i32);

impl Encodable for VarInt {
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        let bytes = leb128::build_var_int(self.0);
        writer.write_all(&bytes).expect("could not write all bytes");

        bytes.len()
    }
}
impl Decodable for VarInt {
    fn decode<T: Read>(reader: &mut T) -> CResult<Self> {
        Ok(VarInt(read_var_int(reader)?.1))
    }
}
impl From<i32> for VarInt {
    fn from(value: i32) -> Self {
        Self(value)
    }
}
impl From<VarInt> for i32 {
    fn from(value: VarInt) -> Self {
        value.0
    }
}
impl VarInt {
    fn inner(self) -> i32 {
        self.0
    }
}

impl Encodable for String {
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        let mut written = 0;

        written += VarInt(self.len() as _).encode(writer);
        writer
            .write_all(self.as_bytes())
            .expect("could not write to buffer");
        written += self.len();

        written
    }
}
impl Decodable for String {
    fn decode<T: Read>(reader: &mut T) -> CResult<Self> {
        let length = VarInt::decode(reader)?.inner() as usize;
        let mut buf = vec![0; length];
        reader.read_exact(&mut buf)?;

        Ok(String::from_utf8(buf)?)
    }
}

#[derive(Encodable, Decodable, PartialEq, Eq, Clone)]
struct Chat {
    buf: String,
}

#[derive(Encodable, Decodable, PartialEq, Eq, Clone)]
struct Identifier {
    buf: String,
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::protocol::ReceivedPacket;

    use super::*;

    #[test]
    fn encode_decode_test() {
        #[derive(Encodable, Decodable, Debug, PartialEq, Eq, Clone)]
        struct TestType {
            x: u8,
            y: i8,
            test: i32,
            var: VarInt,
            s: String,
        }

        let tt = TestType {
            x: 3,
            y: -5,
            test: 50,
            var: (-100).into(),
            s: "helloworld".into(),
        };

        let bytes = tt.clone().to_bytes();

        let rp = ReceivedPacket { buf: Cursor::new(bytes.into_boxed_slice()) };

        let r = TestType::from_packet(rp);

        assert_eq!(r.unwrap(), tt);
    }
}
