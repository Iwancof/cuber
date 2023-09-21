use std::{
    io::{Read, Write},
    marker::PhantomData,
    slice::{Iter, IterMut},
};

use crate::protocol::{Decodable, Encodable};

use super::{CResult, VarInt};
pub trait ArrayLength: Sized {
    fn from_len(len: usize) -> Self;
    fn got_element(&mut self);
    fn has_next(&self) -> bool;
    fn is_end(&self) -> bool {
        !self.has_next()
    }
}

impl ArrayLength for VarInt {
    fn from_len(len: usize) -> Self {
        VarInt(len as _)
    }
    fn got_element(&mut self) {
        self.0 -= 1;
    }
    fn has_next(&self) -> bool {
        self.0 > 0
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
    fn decode<T: Read>(_reader: &mut T) -> CResult<Self> {
        Ok(Self { remain: L })
    }
}
impl<const L: usize> ArrayLength for FixedLength<L> {
    fn from_len(len: usize) -> Self {
        if len != L {
            panic!(
                "Fixed array length mismatch: expected {}, but got {}",
                L, len
            );
        }
        Self { remain: L }
    }
    fn got_element(&mut self) {
        self.remain -= 1;
    }
    fn has_next(&self) -> bool {
        self.remain > 0
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
    fn decode<T: Read>(_reader: &mut T) -> CResult<Self> {
        Ok(Self)
    }
}

impl ArrayLength for PacketInferredInBytes {
    fn from_len(len: usize) -> Self {
        Self
    }
    fn got_element(&mut self) {
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

        let l = L::from_len(self.inner.len());
        written += l.encode(writer);
        written += self.iter().map(|inner| inner.encode(writer)).sum::<usize>();

        written
    }
}
impl<L, Inner> Decodable for Array<L, Inner>
where
    Inner: Decodable,
    L: Decodable + ArrayLength,
{
    fn decode<T: Read>(reader: &mut T) -> CResult<Self> {
        let mut remain_checker: L = L::decode(reader)?;
        let mut inner = Vec::new();

        while let Ok(element) = Inner::decode(reader) {
            inner.push(element);
            remain_checker.got_element();

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
    use crate::protocol::primitive::VarInt;
    use std::io::Cursor;

    #[test]
    fn array_varint_encode() {
        let mut buf = Vec::new();
        Array::<VarInt, u8>::from(vec![1, 2, 3, 4, 5]).encode(&mut buf);

        assert_eq!(buf, vec![5, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn array_varint_decode() {
        let mut buf = Cursor::new(vec![5, 1, 2, 3, 4, 5]);
        let decoded = Array::<VarInt, u8>::decode(&mut buf).unwrap();

        assert_eq!(decoded.inner, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    #[should_panic]
    fn array_varint_decode_panic() {
        let mut buf = Cursor::new(vec![5, 1, 2, 3, 4]);
        let _decoded = Array::<VarInt, u8>::decode(&mut buf).unwrap();
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