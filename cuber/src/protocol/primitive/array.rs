use std::{
    io::{Read, Write},
    marker::PhantomData,
    slice::{Iter, IterMut},
};

use deriver::{Decodable, Encodable};

use crate::protocol::{Decodable, Encodable};
use super::VarInt;

use anyhow::Result;

pub trait ArrayLength: Sized {
    fn from(write_object: usize, write_bytes: usize) -> Self;
    fn got_element(&mut self, read_object: usize, read_bytes: usize);
    fn has_next(&self) -> bool;
    fn is_end(&self) -> bool {
        !self.has_next()
    }
}

#[derive(Encodable, Decodable, PartialEq, Eq, Clone, Copy, Hash)]
pub struct VarIntLength {
    len: VarInt,
}

impl ArrayLength for VarIntLength {
    fn from(write_object: usize, _write_bytes: usize) -> Self {
        VarIntLength {
            len: VarInt(write_object as _),
        }
    }
    fn got_element(&mut self, read_object: usize, _read_bytes: usize) {
        self.len.0 -= read_object as i32;
    }
    fn has_next(&self) -> bool {
        self.len.0 > 0
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct FixedLength<const L: usize> {
    remain: usize,
}

impl<const L: usize> Encodable for FixedLength<L> {
    fn encode<T: Write>(&self, _writer: &mut T) -> usize {
        0
    }
}
impl<const L: usize> Decodable for FixedLength<L> {
    fn decode<T: Read>(_reader: &mut T) -> Result<Self> {
        Ok(Self { remain: L })
    }
}
impl<const L: usize> ArrayLength for FixedLength<L> {
    fn from(write_object: usize, _write_bytes: usize) -> Self {
        if write_object != L {
            panic!(
                "Fixed array length mismatch: expected {}, but got {}",
                L, write_object
            );
        }
        Self { remain: L }
    }
    fn got_element(&mut self, read_object: usize, _read_bytes: usize) {
        self.remain -= read_object;
    }
    fn has_next(&self) -> bool {
        self.remain > 0
    }
}

#[derive(Encodable, Decodable, PartialEq, Eq, Clone, Copy, Hash)]
pub struct VarIntLengthInBytes {
    len: VarInt,
}

impl ArrayLength for VarIntLengthInBytes {
    fn from(_write_object: usize, write_bytes: usize) -> Self {
        VarIntLengthInBytes {
            len: VarInt(write_bytes as _),
        }
    }
    fn got_element(&mut self, _read_object: usize, read_bytes: usize) {
        self.len.0 -= read_bytes as i32;
    }
    fn has_next(&self) -> bool {
        self.len.0 > 0
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct PacketInferredInBytes;
impl Encodable for PacketInferredInBytes {
    fn encode<T: Write>(&self, _writer: &mut T) -> usize {
        0
    }
}
impl Decodable for PacketInferredInBytes {
    fn decode<T: Read>(_reader: &mut T) -> Result<Self> {
        Ok(Self)
    }
}

impl ArrayLength for PacketInferredInBytes {
    fn from(_write_object: usize, _write_bytes: usize) -> Self {
        Self
    }
    fn got_element(&mut self, _read_object: usize, _read_bytes: usize) {
        // ah, ok.
    }
    // PacketInferred では、要素数がパケット依存なので has_next = true にしつつ is_ned = true にする
    fn has_next(&self) -> bool {
        true
    }
    fn is_end(&self) -> bool {
        true
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
    L: Encodable + ArrayLength,
{
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        let mut written = 0;
        let mut tmp_buf = Vec::with_capacity(core::mem::size_of::<Inner>() * self.inner.len()); // for performance

        let object_num = self.inner.len();
        let wrote_num = self
            .iter()
            .map(|inner| inner.encode(&mut tmp_buf))
            .sum::<usize>();

        let l = L::from(object_num, wrote_num);
        written += l.encode(writer);
        written += tmp_buf.len();

        writer.write_all(&tmp_buf).unwrap();

        written
    }
}
impl<L, Inner> Decodable for Array<L, Inner>
where
    Inner: Decodable,
    L: Decodable + ArrayLength,
{
    fn decode<Outer: Read>(reader: &mut Outer) -> Result<Self> {
        struct ReadCountWrapper<Inner> {
            inner: Inner,
            count: usize,
        }
        impl<Inner> Read for ReadCountWrapper<Inner>
        where
            Inner: Read,
        {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                let read = self.inner.read(buf)?;
                self.count += read;
                Ok(read)
            }
        }
        let reader = &mut ReadCountWrapper {
            inner: reader,
            count: 0,
        };

        let mut remain_checker: L = L::decode(reader)?;
        let mut inner = Vec::new();

        while let Ok(element) = Inner::decode(reader) {
            inner.push(element);
            remain_checker.got_element(1, reader.count);

            if !remain_checker.has_next() {
                break;
            }
        }

        assert!(remain_checker.is_end());

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

impl<L, Inner> From<&[Inner]> for Array<L, Inner>
where
    Inner: Clone,
{
    fn from(value: &[Inner]) -> Self {
        Self {
            inner: value.to_vec(),
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
    use super::*;
    use std::io::Cursor;

    #[test]
    fn array_varint_encode() {
        let mut buf = Vec::new();
        Array::<VarIntLength, u8>::from(vec![1, 2, 3, 4, 5]).encode(&mut buf);

        assert_eq!(buf, vec![5, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn array_varint_decode() {
        let mut buf = Cursor::new(vec![5, 1, 2, 3, 4, 5]);
        let decoded = Array::<VarIntLength, u8>::decode(&mut buf).unwrap();

        assert_eq!(decoded.inner, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    #[should_panic]
    fn array_varint_decode_panic() {
        let mut buf = Cursor::new(vec![5, 1, 2, 3, 4]);
        let _decoded = Array::<VarIntLength, u8>::decode(&mut buf).unwrap();
    }

    #[test]
    fn array_varint_inbytes_encode() {
        #[derive(Encodable, Decodable, Debug, PartialEq, Eq, Clone, Copy, Hash)]
        pub struct TestStruct {
            pub a: u8,
            pub b: u8,
        }

        let mut buf = Vec::new();
        Array::<VarIntLengthInBytes, TestStruct>::from(vec![
            TestStruct { a: 1, b: 2 },
            TestStruct { a: 3, b: 4 },
        ])
        .encode(&mut buf);

        assert_eq!(buf, vec![4, 1, 2, 3, 4]);
    }

    #[test]
    fn array_varint_inbytes_decode() {
        #[derive(Encodable, Decodable, Debug, PartialEq, Eq, Clone, Copy, Hash)]
        pub struct TestStruct {
            pub a: u8,
            pub b: u8,
        }

        let mut buf = Cursor::new(vec![4, 1, 2, 3, 4]);
        let decoded = Array::<VarIntLengthInBytes, TestStruct>::decode(&mut buf).unwrap();

        assert_eq!(
            decoded.inner,
            vec![TestStruct { a: 1, b: 2 }, TestStruct { a: 3, b: 4 }]
        );
    }

    #[test]
    #[should_panic]
    fn array_varint_inbytes_decode_panic() {
        #[derive(Encodable, Decodable, Debug, PartialEq, Eq, Clone, Copy, Hash)]
        pub struct TestStruct {
            pub a: u8,
            pub b: u8,
        }

        let mut buf = Cursor::new(vec![4, 1, 2, 3]);
        let _decoded = Array::<VarIntLengthInBytes, TestStruct>::decode(&mut buf).unwrap();
    }

    #[test]
    fn array_fixed_encode() {
        let mut buf = Vec::new();
        Array::<FixedLength<5>, u8>::from(vec![1, 2, 3, 4, 5]).encode(&mut buf);

        assert_eq!(buf, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn array_fixed_decode() {
        let mut buf = Cursor::new(vec![1, 2, 3, 4, 5]);
        let decoded = Array::<FixedLength<5>, u8>::decode(&mut buf).unwrap();

        assert_eq!(decoded.inner, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    #[should_panic]
    fn array_fixed_decode_panic() {
        let mut buf = Cursor::new(vec![1, 2, 3, 4]);
        let _decoded = Array::<FixedLength<5>, u8>::decode(&mut buf).unwrap();
    }

    #[test]
    fn array_packet_inferred_encode() {
        let mut buf = Vec::new();
        Array::<PacketInferredInBytes, u8>::from(vec![1, 2, 3, 4, 5]).encode(&mut buf);

        assert_eq!(buf, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn array_packet_inferred_decode() {
        let mut buf = Cursor::new(vec![1, 2, 3, 4, 5]);
        let decoded = Array::<PacketInferredInBytes, u8>::decode(&mut buf).unwrap();

        assert_eq!(decoded.inner, vec![1, 2, 3, 4, 5]);
    }
}
