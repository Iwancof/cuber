pub mod array;
pub mod leb128;

use array::Array;
use leb128::read_var_int;

use self::array::VarIntLength;

use super::{Decodable, Encodable};

use deriver::{Decodable, Encodable};

use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use nbt::Blob;
use std::io::{Read, Write};
use uuid::Uuid;

use anyhow::{Result, Context as _, bail, ensure};

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
        const fn get_size<T>(_: &T) -> usize {
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
        $reader.read_u8()
    };
    ($reader: ident, read_i8) => {
        $reader.read_i8()
    };
    ($reader: ident, $method: ident) => {
        $reader.$method::<NetworkEndian>()
    };
}

macro_rules! define_prim {
    ($type: ty, $write_method: ident, $read_method: ident) => {
        impl Encodable for $type {
            fn encode<T: Write>(&self, writer: &mut T) -> usize {
                write_primitive!(writer, $write_method, *self)
            }
        }
        impl Decodable for $type {
            fn decode<T: Read>(reader: &mut T) -> Result<Self> {
                Ok(read_primitive!(reader, $read_method).with_context(|| {
                    format!("could not read data. {}()", stringify!($read_method))
                })?)
            }
        }
    };
}

define_prim!(i8, write_i8, read_i8);
define_prim!(u8, write_u8, read_u8);
define_prim!(i16, write_i16, read_i16);
define_prim!(u16, write_u16, read_u16);
define_prim!(i32, write_i32, read_i32);
define_prim!(u32, write_u32, read_u32);
define_prim!(i64, write_i64, read_i64);
define_prim!(u64, write_u64, read_u64);
define_prim!(i128, write_i128, read_i128);
define_prim!(u128, write_u128, read_u128);
define_prim!(f32, write_f32, read_f32);
define_prim!(f64, write_f64, read_f64);

impl Encodable for bool {
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        (if *self { 1_u8 } else { 0_u8 }).encode(writer)
    }
}
impl Decodable for bool {
    fn decode<T: Read>(reader: &mut T) -> Result<Self> {
        match u8::decode(reader)? {
            0 => Ok(false),
            1 => Ok(true),
            x => bail!("invalid bool value: {}", x),
        }
    }
}

#[derive(Encodable, Decodable, Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Angle {
    pub value: u8,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct VarInt(pub i32);

impl Encodable for VarInt {
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        let bytes = leb128::build_var_int(self.0);
        writer.write_all(&bytes).expect("could not write all bytes");

        bytes.len()
    }
}
impl Decodable for VarInt {
    fn decode<T: Read>(reader: &mut T) -> Result<Self> {
        Ok(VarInt(read_var_int(reader).context("could not read var int")?.1))
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
    fn decode<T: Read>(reader: &mut T) -> Result<Self> {
        let buf = Array::<VarIntLength, u8>::decode(reader).context("could not read string length")?.inner;
        Ok(String::from_utf8(buf)?)
    }
}

#[derive(Encodable, Decodable, Debug, PartialEq, Eq, Clone, Hash)]
pub struct Chat {
    buf: String,
}

#[derive(Encodable, Decodable, Debug, PartialEq, Eq, Clone, Hash)]
pub struct Identifier {
    buf: String,
}
impl From<String> for Identifier {
    fn from(value: String) -> Self {
        Self { buf: value }
    }
}
impl From<&str> for Identifier {
    fn from(value: &str) -> Self {
        Self {
            buf: value.to_string(),
        }
    }
}
impl From<Identifier> for String {
    fn from(value: Identifier) -> Self {
        value.buf
    }
}

impl Encodable for Uuid {
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        self.as_u128().encode(writer)
    }
}

impl Decodable for Uuid {
    fn decode<T: Read>(reader: &mut T) -> Result<Self> {
        Ok(Self::from_u128_le(u128::decode(reader).context("could not read uuid")?))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    x: i32, // 26 bits
    y: i32, // 12 bits
    z: i32, // 26 bits
}

impl Position {
    fn is_valid(&self) -> bool {
        (-2_i32).pow(25) <= self.x
            && self.x < 2_i32.pow(25)
            && (-2_i32).pow(11) <= self.y
            && self.y < 2_i32.pow(11)
            && (-2_i32).pow(25) <= self.z
            && self.z < 2_i32.pow(25)
    }
    pub fn new(x: i32, y: i32, z: i32) -> Result<Self> {
        let pos = Self { x, y, z };
        ensure!(pos.is_valid(), "invalid position: {:?}", pos);
        Ok(pos)
    }
    pub fn set_x(&mut self, x: i32) -> &mut Self {
        self.x = x;
        self
    }
    pub fn set_y(&mut self, y: i32) -> &mut Self {
        self.y = y;
        self
    }
    pub fn set_z(&mut self, z: i32) -> &mut Self {
        self.z = z;
        self
    }
    pub fn pack(&self) -> i64 {
        let mut packed = 0_i64;
        packed |= (self.x as i64 & 0x3ffffff) << 38;
        packed |= (self.z as i64 & 0x3ffffff) << 12;
        packed |= self.y as i64 & 0xfff;
        packed
    }
    pub fn unpack(packed: i64) -> Result<Self> {
        Self::new(
            ((packed >> 38) & 0x3ffffff) as i32,
            ((packed << 52) >> 52) as i32,
            ((packed << 26) >> 38) as i32,
        )
    }
}

impl Encodable for Position {
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        self.pack().encode(writer)
    }
}

impl Decodable for Position {
    fn decode<T: Read>(reader: &mut T) -> Result<Self> {
        Self::unpack(i64::decode(reader).context("could not read position")?)
    }
}

impl Encodable for Blob {
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        self.to_writer(writer).expect("could not write nbt data");
        self.len_bytes()
    }
}

impl Decodable for Blob {
    fn decode<T: Read>(reader: &mut T) -> Result<Self> {
        Ok(Self::from_reader(reader)?)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct BoolConditional<T>(pub Option<T>);

impl<Inner> Encodable for BoolConditional<Inner>
where
    Inner: Encodable,
{
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        match &self.0 {
            Some(obj) => {
                let mut written = 0;
                written += true.encode(writer);
                written += obj.encode(writer);

                return written;
            }
            None => false.encode(writer),
        }
    }
}

impl<Inner> Decodable for BoolConditional<Inner>
where
    Inner: Decodable,
{
    fn decode<T: Read>(reader: &mut T) -> Result<Self> {
        if !bool::decode(reader).context("could not read bool in BoolConditional")? {
            return Ok(Self(None));
        }
        return Ok(Self(Some(Inner::decode(reader).context("could not read inner value in BoolConditional")?)));
    }
}
impl<Inner> From<Option<Inner>> for BoolConditional<Inner> {
    fn from(value: Option<Inner>) -> Self {
        Self(value)
    }
}
impl<Inner> From<BoolConditional<Inner>> for Option<Inner> {
    fn from(value: BoolConditional<Inner>) -> Self {
        value.0
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Todo;
impl Encodable for Todo {
    fn encode<T: Write>(&self, _writer: &mut T) -> usize {
        todo!()
    }
}
impl Decodable for Todo {
    fn decode<T: Read>(_reader: &mut T) -> Result<Self> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufWriter, Cursor};

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

        let buf = Vec::new();
        let mut buf_writer = BufWriter::new(buf);
        tt.clone().encode(&mut buf_writer);

        let mut rp = ReceivedPacket {
            buf: Cursor::new(buf_writer.into_inner().unwrap().into_boxed_slice()),
        };

        let r = TestType::decode(&mut rp);

        assert_eq!(r.unwrap(), tt);
    }

    #[test]
    fn f32_encode() {
        let mut buf = Vec::new();
        0.1_f32.encode(&mut buf);

        // 0.1 = 0x3dcccccd
        assert_eq!(buf, vec![0x3d, 0xcc, 0xcc, 0xcd]);
    }

    #[test]
    fn f32_decode() {
        let mut buf = Cursor::new(vec![0x3d, 0xcc, 0xcc, 0xcd]);
        assert_eq!(f32::decode(&mut buf).unwrap(), 0.1_f32);
    }

    #[test]
    fn bool_encode() {
        let mut buf = Vec::new();
        true.encode(&mut buf);
        false.encode(&mut buf);

        assert_eq!(buf, vec![1, 0]);
    }

    #[test]
    fn bool_decode() {
        let mut buf = Cursor::new(vec![1, 0, 10]);
        assert_eq!(bool::decode(&mut buf).unwrap(), true);
        assert_eq!(bool::decode(&mut buf).unwrap(), false);
        bool::decode(&mut buf).unwrap_err();
    }

    #[test]
    fn string_encode() {
        let mut buf = Vec::new();
        String::from("hello").encode(&mut buf);

        assert_eq!(buf, vec![5, 104, 101, 108, 108, 111]);
    }

    #[test]
    fn string_decode() {
        let mut buf = Cursor::new(vec![5, 104, 101, 108, 108, 111]);
        assert_eq!(
            String::decode(&mut buf).unwrap(),
            String::from("hello").to_string()
        );
    }

    #[test]
    fn bool_condition_encode() {
        let mut buf = Vec::new();
        BoolConditional::<u8>(Some(5_u8)).encode(&mut buf);
        BoolConditional::<u8>(None).encode(&mut buf);

        assert_eq!(buf, vec![1, 5, 0]);
    }

    #[test]
    fn bool_condition_decode() {
        let mut buf = Cursor::new(vec![1, 5, 0]);
        assert_eq!(
            BoolConditional::<u8>::decode(&mut buf).unwrap(),
            BoolConditional(Some(5_u8))
        );
        assert_eq!(
            BoolConditional::<u8>::decode(&mut buf).unwrap(),
            BoolConditional(None)
        );
    }

    #[test]
    fn position_unpack() {
        let raw = 0b01000110000001110110001100_10110000010101101101001000_001100111111;
        let Position { x, y, z } = Position::unpack(raw).unwrap();

        assert_eq!(x, 18357644);
        assert_eq!(y, 831);
        assert_eq!(z, -20882616);
    }

    #[test]
    fn position_pack() {
        let pos = Position::new(18357644, 831, -20882616).unwrap();
        assert_eq!(
            pos.pack(),
            0b01000110000001110110001100_10110000010101101101001000_001100111111
        );
    }
}
