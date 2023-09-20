pub mod leb128;

use self::leb128::read_var_int;

use super::CResult;
use super::{Decodable, Encodable};

use deriver::{Decodable, Encodable};

use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use nbt::Blob;
use std::io::{ErrorKind, Read, Write};
use std::marker::PhantomData;
use std::slice::{Iter, IterMut};
use uuid::Uuid;

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
            fn decode<T: Read>(reader: &mut T) -> CResult<Self> {
                Ok(read_primitive!(reader, $read_method)?)
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
    fn decode<T: Read>(reader: &mut T) -> CResult<Self> {
        Ok(u8::decode(reader)? == 1)
    }
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
impl TryInto<usize> for VarInt {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<usize, Self::Error> {
        if self.0 < 0 {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                format!("could not convert to usize: {}", self.0),
            )
            .into());
        }
        Ok(self.0 as usize)
    }
}
impl From<usize> for VarInt {
    fn from(value: usize) -> Self {
        Self(value as _)
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
    fn decode<T: Read>(reader: &mut T) -> CResult<Self> {
        Ok(Self::from_u128_le(u128::decode(reader)?))
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
    pub fn new(x: i32, y: i32, z: i32) -> CResult<Self> {
        let pos = Self { x, y, z };
        if !pos.is_valid() {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                format!("invalid position: {:?}", pos),
            )
            .into());
        }
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
        packed |= (self.y as i64 & 0xfff) << 26;
        packed |= self.z as i64 & 0x3ffffff;
        packed
    }
    pub fn unpack(packed: i64) -> CResult<Self> {
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
    fn decode<T: Read>(reader: &mut T) -> CResult<Self> {
        Self::unpack(i64::decode(reader)?)
    }
}

impl Encodable for Blob {
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        self.to_writer(writer).expect("could not write nbt data");
        self.len_bytes()
    }
}

impl Decodable for Blob {
    fn decode<T: Read>(reader: &mut T) -> CResult<Self> {
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
    fn decode<T: Read>(reader: &mut T) -> CResult<Self> {
        if !bool::decode(reader)? {
            return Ok(Self(None));
        }
        return Ok(Self(Some(Inner::decode(reader)?)));
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

impl<const L: usize, Inner> Encodable for [Inner; L]
where
    Inner: Encodable,
{
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        self.iter().map(|inner| inner.encode(writer)).sum()
    }
}

impl<const L: usize, Inner> Decodable for [Inner; L]
where
    Inner: Decodable,
{
    fn decode<T: Read>(reader: &mut T) -> CResult<Self> {
        Ok(array_macro::array![_ => Inner::decode(reader)?; L])
    }
}

#[derive(PartialEq, Eq, Clone, Hash)]
pub struct Array<L, T> {
    pub inner: Vec<T>,
    pub _phantom: PhantomData<fn(L) -> ()>,
}
impl<L, Inner> Encodable for Array<L, Inner>
where
    Inner: Encodable,
    L: Encodable + From<usize>,
{
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        let mut written = 0;

        let l = L::from(self.inner.len());
        written += l.encode(writer);
        written += self.iter().map(|inner| inner.encode(writer)).sum::<usize>();

        written
    }
}
impl<L, Inner> Decodable for Array<L, Inner>
where
    Inner: Decodable,
    L: Decodable + TryInto<usize>,
    <L as TryInto<usize>>::Error: Into<anyhow::Error>,
{
    fn decode<T: Read>(reader: &mut T) -> CResult<Self> {
        let length: usize = L::decode(reader)?.try_into().map_err(|e| e.into())?;
        let mut inner = Vec::with_capacity(length);

        for _ in 0..length {
            inner.push(Inner::decode(reader)?);
        }

        Ok(Self {
            inner,
            _phantom: PhantomData,
        })
    }
}

impl<L, Inner> std::fmt::Debug for Array<L, Inner>
where
    Inner: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::any::type_name;
        f.debug_struct(&format!(
            "Array<{}, {}>",
            type_name::<L>(),
            type_name::<Inner>()
        ))
        .field("inner", &&self.inner)
        .finish()
    }
}
impl<L, Inner> From<Vec<Inner>> for Array<L, Inner> {
    fn from(value: Vec<Inner>) -> Self {
        Self {
            inner: value,
            _phantom: PhantomData,
        }
    }
}
impl<L, Inner> Array<L, Inner> {
    #[allow(unused)]
    pub fn iter(&self) -> Iter<Inner> {
        self.inner.iter()
    }
    #[allow(unused)]
    pub fn iter_mut(&mut self) -> IterMut<Inner> {
        self.inner.iter_mut()
    }
    #[allow(unused)]
    pub fn from_fixed<const LENGTH: usize>(fixed: [Inner; LENGTH]) -> Self
    where
        Inner: Clone,
    {
        Self {
            inner: fixed.to_vec(),
            _phantom: PhantomData,
        }
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
    fn decode<T: Read>(_reader: &mut T) -> CResult<Self> {
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
    fn position_unpack() {
        let raw = 0b01000110000001110110001100_10110000010101101101001000_001100111111;
        let Position { x, y, z } = Position::unpack(raw).unwrap();

        assert_eq!(x, 18357644);
        assert_eq!(y, 831);
        assert_eq!(z, -20882616);
    }

    fn position_pack() {
        let pos = Position::new(18357644, 831, -20882616).unwrap();
        assert_eq!(
            pos.pack(),
            0b01000110000001110110001100_10110000010101101101001000_001100111111
        );
    }
}
