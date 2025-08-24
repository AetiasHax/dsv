use std::{borrow::Cow, ops::Range};

use bitvec::{order::Lsb0, slice::BitSlice, vec::BitVec};
use dsv_core::state::State;

use crate::util::bitvec::BitVecExt;

#[derive(Clone)]
pub struct TypeInstance<'a> {
    ty: &'a type_crawler::TypeKind,
    address: u32,
    bit_field_range: Option<Range<u8>>,
    data: Cow<'a, [u8]>,
}

pub struct TypeInstanceOptions<'a> {
    pub ty: &'a type_crawler::TypeKind,
    pub address: u32,
    pub bit_field_range: Option<Range<u8>>,
    pub data: Cow<'a, [u8]>,
}

impl<'a> TypeInstance<'a> {
    pub fn new(options: TypeInstanceOptions<'a>) -> Self {
        Self {
            ty: options.ty,
            address: options.address,
            bit_field_range: options.bit_field_range,
            data: options.data,
        }
    }

    pub fn slice(
        &'a self,
        types: &type_crawler::Types,
        new_type: &'a type_crawler::TypeKind,
        offset: usize,
        bit_field_range: Option<Range<u8>>,
    ) -> Self {
        let size = if let Some(range) = &bit_field_range {
            (range.end.div_ceil(8) - range.start / 8) as usize
        } else {
            new_type.size(types)
        };

        let start = offset.min(self.data.len());
        let end = (offset + size).min(self.data.len());
        Self {
            ty: new_type,
            address: self.address + offset as u32,
            bit_field_range: bit_field_range.or(self.bit_field_range.clone()),
            data: Cow::Borrowed(&self.data[start..end]),
        }
    }

    pub fn data(&'a self) -> Cow<'a, [u8]> {
        if let Some(range) = &self.bit_field_range {
            let mut bitslice = BitVec::<u8, Lsb0>::from_slice(&self.data);
            let start = range.start as usize;
            bitslice.shift_left(start);
            bitslice.truncate_remove(range.len());
            bitslice.into_vec().into()
        } else {
            Cow::Borrowed(&self.data)
        }
    }

    pub fn data_i64(&self) -> i64 {
        let mut buf = [0u8; 8];
        let data = self.data();
        let data = if data.len() > 8 { &data[..8] } else { &data };
        buf[..data.len()].copy_from_slice(data);
        i64::from_le_bytes(buf)
    }

    pub fn address(&self) -> u32 {
        self.address
    }

    pub fn read_field(
        &'a self,
        types: &'a type_crawler::Types,
        name: &str,
    ) -> Option<TypeInstance<'a>> {
        match self.ty {
            type_crawler::TypeKind::Struct(struct_decl) => {
                let field = struct_decl.get_field(types, name)?;
                let ty = field.kind().expand_named(types)?;
                let offset = field.offset_bytes();
                let bit_field_range = if let Some(width) = field.bit_field_width() {
                    let start = (field.offset_bits() - offset * 8) as u8;
                    Some(start..start + width)
                } else {
                    None
                };
                Some(self.slice(types, ty, offset, bit_field_range))
            }
            type_crawler::TypeKind::Union(union_decl) => {
                let field = union_decl.get_field(name)?;
                let ty = field.kind().expand_named(types)?;
                let bit_field_range = field.bit_field_width().map(|width| 0..width);
                Some(self.slice(types, ty, 0, bit_field_range))
            }
            _ => None,
        }
    }

    pub fn as_int<T>(&self, types: &type_crawler::Types) -> Option<T>
    where
        T: Copy + TryFrom<i64>,
    {
        let value = self.ty.read_int_value(types, self)?;
        T::try_from(value).ok()
    }

    pub fn read_int_field<T>(&self, types: &type_crawler::Types, name: &str) -> Option<T>
    where
        T: Copy + TryFrom<i64>,
    {
        self.read_field(types, name).and_then(|field| field.as_int::<T>(types))
    }

    pub fn ty(&self) -> &'a type_crawler::TypeKind {
        self.ty
    }

    pub fn bit_field_range(&self) -> Option<&Range<u8>> {
        self.bit_field_range.as_ref()
    }

    pub fn write(&self, state: &mut State, data: Vec<u8>) {
        if let Some(range) = &self.bit_field_range {
            let mut data_bits: BitVec<u8, Lsb0> = BitVec::from_vec(data);
            data_bits.truncate_remove(range.len());
            let end_bit = range.len().next_multiple_of(8);
            data_bits.resize(end_bit, false);
            debug_assert_eq!(data_bits.len() / 8, self.data.len());
            data_bits.shift_right(range.start as usize);

            let current_bits = BitSlice::from_slice(&self.data);
            data_bits[0..range.start as usize]
                .copy_from_bitslice(&current_bits[0..range.start as usize]);
            data_bits[range.end as usize..end_bit]
                .copy_from_bitslice(&current_bits[range.end as usize..end_bit]);

            state.request_write(self.address, data_bits.into_vec());
        } else {
            state.request_write(self.address, data);
        }
    }

    pub fn with_type(self, ty: &'a type_crawler::TypeKind) -> Self {
        Self {
            ty,
            address: self.address,
            bit_field_range: self.bit_field_range,
            data: self.data,
        }
    }
}

pub trait ReadIntValue {
    fn read_int_value(&self, types: &type_crawler::Types, instance: &TypeInstance) -> Option<i64>;
}

impl ReadIntValue for type_crawler::TypeKind {
    fn read_int_value(&self, types: &type_crawler::Types, instance: &TypeInstance) -> Option<i64> {
        match self {
            type_crawler::TypeKind::USize { .. } => Some(instance.data_i64() as u32 as i64),
            type_crawler::TypeKind::SSize { .. } => Some(instance.data_i64() as i32 as i64),
            type_crawler::TypeKind::U64 => Some(instance.data_i64() as u64 as i64),
            type_crawler::TypeKind::U32 => Some(instance.data_i64() as u32 as i64),
            type_crawler::TypeKind::U16 => Some(instance.data_i64() as u16 as i64),
            type_crawler::TypeKind::Bool | type_crawler::TypeKind::U8 => {
                Some(instance.data_i64() as u8 as i64)
            }
            type_crawler::TypeKind::S64 => Some(instance.data_i64()),
            type_crawler::TypeKind::S32 => Some(instance.data_i64() as i32 as i64),
            type_crawler::TypeKind::S16 => Some(instance.data_i64() as i16 as i64),
            type_crawler::TypeKind::S8 => Some(instance.data_i64() as i8 as i64),
            type_crawler::TypeKind::F32 => None,
            type_crawler::TypeKind::F64 => None,
            type_crawler::TypeKind::LongDouble { .. } => None,
            type_crawler::TypeKind::Char16 => None,
            type_crawler::TypeKind::Char32 => None,
            type_crawler::TypeKind::WChar { .. } => None,
            type_crawler::TypeKind::Void => None,
            type_crawler::TypeKind::Reference { .. }
            | type_crawler::TypeKind::Pointer { .. }
            | type_crawler::TypeKind::MemberPointer { .. } => {
                Some(instance.data_i64() as u32 as i64)
            }
            type_crawler::TypeKind::Array { .. } => None,
            type_crawler::TypeKind::Function { .. } => None,
            type_crawler::TypeKind::Struct(_) => None,
            type_crawler::TypeKind::Class(_) => None,
            type_crawler::TypeKind::Union(_) => None,
            type_crawler::TypeKind::Enum(enum_decl) => match enum_decl.size() {
                1 => Some(instance.data_i64() as i8 as i64),
                2 => Some(instance.data_i64() as i16 as i64),
                4 => Some(instance.data_i64() as i32 as i64),
                8 => Some(instance.data_i64()),
                _ => None,
            },
            type_crawler::TypeKind::Typedef(typedef) => {
                typedef.underlying_type().read_int_value(types, instance)
            }
            type_crawler::TypeKind::Named(name) => {
                if let Some(ty) = types.get(name) {
                    ty.read_int_value(types, instance)
                } else {
                    None
                }
            }
        }
    }
}
