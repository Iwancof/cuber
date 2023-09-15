pub mod leb128;

use super::Encodable;
pub use super::{SResult, AResult};

use std::io::Write;
use byteorder::{NetworkEndian, WriteBytesExt};

macro_rules! write_primitive {
    ($writer: ident, write_u8, $value: expr) => {
        {
            $writer.write_u8($value);
            1
        }
    };
    ($writer: ident, write_i8, $value: expr) => {
        {
            $writer.write_i8($value);
            1
        }
    };
    ($writer: ident, $method: ident, $value: expr) => {
        {
            fn get_size<T>(_: &T) -> usize {
                std::mem::size_of::<T>()
            }

            let bind = $value;
            let len = get_size(&bind);
            $writer.$method::<NetworkEndian>(bind);

            len
        }
        
    };
}

impl Encodable for bool {
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        write_primitive!(writer, write_u8, if *self { 1 } else { 0 })
    }
}

macro_rules! define_prim {
    ($type: ty, $method: ident) => {
        impl Encodable for $type {
            fn encode<T: Write>(&self, writer: &mut T) -> usize {
                write_primitive!(writer, $method, *self)
            }
        }
    };
}

define_prim!(i8, write_i8);
define_prim!(u8, write_u8);
define_prim!(i16, write_i16);
define_prim!(u16, write_u16);
define_prim!(i32, write_i32);
define_prim!(i64, write_i64);
define_prim!(f32, write_f32);
define_prim!(f64, write_f64);

struct VarInt(i32);
impl Encodable for VarInt {
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        let bytes = leb128::build_var_int(self.0);
        writer.write(&bytes);

        bytes.len()
    }
}

impl Encodable for String {
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        let mut written = 0;

        written += VarInt(self.len() as _).encode(writer);
        writer.write_all(&self.into_bytes()).expect("could not write to buffer");
        written += self.len();

        written
    }
}

struct Chat(String);
impl Encodable for Chat {
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        self.0.encode(writer)
    }
}

struct Identifier(String);
impl Encodable for Identifier {
    fn encode<T: Write>(&self, writer: &mut T) -> usize {
        self.0.encode(writer)
    }
}
