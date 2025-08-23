use std::{borrow::Cow, ops::Range};

use dzv_core::state::State;
use eframe::egui;

use crate::{
    ui::type_decl::{AsDataWidget, DataWidget},
    util::vec::VecExt,
};

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
    ) -> Self {
        let start = offset.min(self.data.len());
        let end = (offset + new_type.size(types)).min(self.data.len());
        Self {
            ty: new_type,
            address: self.address + offset as u32,
            bit_field_range: self.bit_field_range.clone(),
            data: Cow::Borrowed(&self.data[start..end]),
        }
    }

    pub fn set_bit_field_range(&mut self, range: Range<u8>) {
        self.bit_field_range = Some(range);
    }

    pub fn data(&self) -> &[u8] {
        &self.data
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
                Some(self.slice(types, ty, offset))
            }
            type_crawler::TypeKind::Union(union_decl) => {
                let field = union_decl.get_field(name)?;
                let ty = field.kind().expand_named(types)?;
                Some(self.slice(types, ty, 0))
            }
            _ => None,
        }
    }

    pub fn as_int<T>(&self, types: &type_crawler::Types) -> Option<T>
    where
        T: Copy + TryFrom<i64>,
    {
        let value = self.ty.read_int_value(types, self)?;
        let value = if let Some(range) = &self.bit_field_range {
            let mask = (1 << range.len()) - 1;
            (value >> range.start) & mask
        } else {
            value
        };
        T::try_from(value).ok()
    }

    pub fn read_int_field<T>(&self, types: &type_crawler::Types, name: &str) -> Option<T>
    where
        T: Copy + TryFrom<i64>,
    {
        self.read_field(types, name).and_then(|field| field.as_int::<T>(types))
    }

    pub fn as_data_widget(
        &'a self,
        ui: &mut egui::Ui,
        types: &'a type_crawler::Types,
    ) -> Box<dyn DataWidget + 'a> {
        self.ty.as_data_widget(ui, types, self.clone())
    }

    pub fn ty(&self) -> &'a type_crawler::TypeKind {
        self.ty
    }

    pub fn bit_field_range(&self) -> Option<&Range<u8>> {
        self.bit_field_range.as_ref()
    }

    pub fn write(&self, state: &mut State, mut data: Vec<u8>) {
        if let Some(range) = &self.bit_field_range {
            data.shift_bits_left(range.start as usize);
            debug_assert_eq!(data.len(), self.data.len());
            data.assign_bits(0, &self.data, 0, range.start as usize).unwrap();
            let end = range.end as usize;
            data.assign_bits(end, &self.data, end, data.len() * 8 - end).unwrap();
            state.request_write(self.address, data);
        } else {
            state.request_write(self.address, data);
        }
    }
}

pub trait ReadIntValue {
    fn read_int_value(&self, types: &type_crawler::Types, instance: &TypeInstance) -> Option<i64>;
}

impl ReadIntValue for type_crawler::TypeKind {
    fn read_int_value(&self, types: &type_crawler::Types, instance: &TypeInstance) -> Option<i64> {
        match self {
            type_crawler::TypeKind::USize { .. } => {
                Some(u32::from_le_bytes(instance.data().try_into().unwrap_or([0; 4])) as i64)
            }
            type_crawler::TypeKind::SSize { .. } => {
                Some(i32::from_le_bytes(instance.data().try_into().unwrap_or([0; 4])) as i64)
            }
            type_crawler::TypeKind::U64 => {
                Some(u64::from_le_bytes(instance.data().try_into().unwrap_or([0; 8])) as i64)
            }
            type_crawler::TypeKind::U32 => {
                Some(u32::from_le_bytes(instance.data().try_into().unwrap_or([0; 4])) as i64)
            }
            type_crawler::TypeKind::U16 => {
                Some(u16::from_le_bytes(instance.data().try_into().unwrap_or([0; 2])) as i64)
            }
            type_crawler::TypeKind::U8 => {
                Some(instance.data().first().copied().unwrap_or(0) as i64)
            }
            type_crawler::TypeKind::S64 => {
                Some(i64::from_le_bytes(instance.data().try_into().unwrap_or([0; 8])))
            }
            type_crawler::TypeKind::S32 => {
                Some(i32::from_le_bytes(instance.data().try_into().unwrap_or([0; 4])) as i64)
            }
            type_crawler::TypeKind::S16 => {
                Some(i16::from_le_bytes(instance.data().try_into().unwrap_or([0; 2])) as i64)
            }
            type_crawler::TypeKind::S8 => {
                Some(instance.data().first().copied().unwrap_or(0) as i8 as i64)
            }
            type_crawler::TypeKind::F32 => None,
            type_crawler::TypeKind::F64 => None,
            type_crawler::TypeKind::LongDouble { .. } => None,
            type_crawler::TypeKind::Char16 => None,
            type_crawler::TypeKind::Char32 => None,
            type_crawler::TypeKind::WChar { .. } => None,
            type_crawler::TypeKind::Bool => None,
            type_crawler::TypeKind::Void => None,
            type_crawler::TypeKind::Reference { .. }
            | type_crawler::TypeKind::Pointer { .. }
            | type_crawler::TypeKind::MemberPointer { .. } => {
                Some(u32::from_le_bytes(instance.data().try_into().unwrap_or([0; 4])) as i64)
            }
            type_crawler::TypeKind::Array { .. } => None,
            type_crawler::TypeKind::Function { .. } => None,
            type_crawler::TypeKind::Struct(_) => None,
            type_crawler::TypeKind::Class(_) => None,
            type_crawler::TypeKind::Union(_) => None,
            type_crawler::TypeKind::Enum(enum_decl) => match enum_decl.size() {
                1 => Some(instance.data().first().copied().unwrap_or(0) as i8 as i64),
                2 => Some(i16::from_le_bytes(instance.data().try_into().unwrap_or([0; 2])) as i64),
                4 => Some(i32::from_le_bytes(instance.data().try_into().unwrap_or([0; 4])) as i64),
                8 => Some(i64::from_le_bytes(instance.data().try_into().unwrap_or([0; 8]))),
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
