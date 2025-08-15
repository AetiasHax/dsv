use std::borrow::Cow;

#[derive(Clone)]
pub struct TypeInstance<'a> {
    address: u32,
    data: Cow<'a, [u8]>,
}

impl<'a> TypeInstance<'a> {
    pub fn new(address: u32, data: impl Into<Cow<'a, [u8]>>) -> Self {
        Self { address, data: data.into() }
    }

    pub fn slice(&'a self, offset: usize, size: usize) -> Self {
        let start = offset.min(self.data.len());
        let end = (offset + size).min(self.data.len());
        Self::new(self.address + offset as u32, &self.data[start..end])
    }

    pub fn get(&self, size: usize) -> &[u8] {
        let end = size.min(self.data.len());
        &self.data[..end]
    }

    pub fn address(&self) -> u32 {
        self.address
    }
}

pub trait ReadField {
    fn read_field<'a>(
        &'_ self,
        types: &type_crawler::Types,
        instance: &'a TypeInstance,
    ) -> (&'_ type_crawler::TypeKind, TypeInstance<'a>);
}

impl ReadField for type_crawler::StructField {
    fn read_field<'a>(
        &'_ self,
        types: &type_crawler::Types,
        instance: &'a TypeInstance,
    ) -> (&'_ type_crawler::TypeKind, TypeInstance<'a>) {
        let offset = self.offset_bytes();
        let size = self.kind().size(types);
        let data = instance.slice(offset, size);
        (self.kind(), data)
    }
}

impl ReadField for type_crawler::Field {
    fn read_field<'a>(
        &'_ self,
        types: &type_crawler::Types,
        instance: &'a TypeInstance,
    ) -> (&'_ type_crawler::TypeKind, TypeInstance<'a>) {
        let size = self.kind().size(types);
        let data = instance.slice(0, size);
        (self.kind(), data)
    }
}

pub trait ReadIntValue {
    fn read_int_value(&self, types: &type_crawler::Types, instance: &TypeInstance) -> Option<i64>;
}

impl ReadIntValue for type_crawler::TypeKind {
    fn read_int_value(&self, types: &type_crawler::Types, instance: &TypeInstance) -> Option<i64> {
        match self {
            type_crawler::TypeKind::USize { .. } => {
                Some(u32::from_le_bytes(instance.get(4).try_into().unwrap_or([0; 4])) as i64)
            }
            type_crawler::TypeKind::SSize { .. } => {
                Some(i32::from_le_bytes(instance.get(4).try_into().unwrap_or([0; 4])) as i64)
            }
            type_crawler::TypeKind::U64 => {
                Some(u64::from_le_bytes(instance.get(8).try_into().unwrap_or([0; 8])) as i64)
            }
            type_crawler::TypeKind::U32 => {
                Some(u32::from_le_bytes(instance.get(4).try_into().unwrap_or([0; 4])) as i64)
            }
            type_crawler::TypeKind::U16 => {
                Some(u16::from_le_bytes(instance.get(2).try_into().unwrap_or([0; 2])) as i64)
            }
            type_crawler::TypeKind::U8 => {
                Some(instance.get(1).first().copied().unwrap_or(0) as i64)
            }
            type_crawler::TypeKind::S64 => {
                Some(i64::from_le_bytes(instance.get(8).try_into().unwrap_or([0; 8])))
            }
            type_crawler::TypeKind::S32 => {
                Some(i32::from_le_bytes(instance.get(4).try_into().unwrap_or([0; 4])) as i64)
            }
            type_crawler::TypeKind::S16 => {
                Some(i16::from_le_bytes(instance.get(2).try_into().unwrap_or([0; 2])) as i64)
            }
            type_crawler::TypeKind::S8 => {
                Some(instance.get(1).first().copied().unwrap_or(0) as i8 as i64)
            }
            type_crawler::TypeKind::Bool => None,
            type_crawler::TypeKind::Void => None,
            type_crawler::TypeKind::Pointer { .. } => None,
            type_crawler::TypeKind::Array { .. } => None,
            type_crawler::TypeKind::Function { .. } => None,
            type_crawler::TypeKind::Struct(_) => None,
            type_crawler::TypeKind::Union(_) => None,
            type_crawler::TypeKind::Named(name) => match types.get(name) {
                Some(type_crawler::TypeDecl::Typedef(typedef)) => {
                    typedef.underlying_type().read_int_value(types, instance)
                }
                Some(type_crawler::TypeDecl::Enum(enum_decl)) => match enum_decl.size() {
                    1 => Some(i8::from_le_bytes(instance.get(1).try_into().unwrap_or([0])) as i64),
                    2 => {
                        Some(i16::from_le_bytes(instance.get(2).try_into().unwrap_or([0; 2])) as i64)
                    }
                    4 => {
                        Some(i32::from_le_bytes(instance.get(4).try_into().unwrap_or([0; 4])) as i64)
                    }
                    8 => Some(i64::from_le_bytes(instance.get(8).try_into().unwrap_or([0; 8]))),
                    _ => None,
                },
                Some(_) => None,
                None => None,
            },
        }
    }
}
