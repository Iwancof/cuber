pub mod leb128;

use self::leb128::read_var_int;

use super::CResult;
use super::{Decodable, Encodable};

use deriver::{Decodable, Encodable};

use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
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
        (if *self { 1 } else { 0 }).encode(writer)
    }
}
impl Decodable for bool {
    fn decode<T: Read>(reader: &mut T) -> CResult<Self> {
        Ok(u8::decode(reader)? == 1)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct VarInt(i32);

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

impl Encodable for Uuid {
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        self.as_u128().encode(writer)
    }
}

impl Decodable for Uuid {
    fn decode<T: Read>(reader: &mut T) -> CResult<Self> {
        let raw = u128::decode(reader)?;
        Ok(Uuid::from_u128_le(raw))
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
}
