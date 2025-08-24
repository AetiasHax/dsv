use std::borrow::Cow;

use dsv_core::state::State;
use eframe::egui::{self, Widget};
use type_crawler::Types;

use crate::{
    ui::columns,
    util::read::{TypeInstance, TypeInstanceOptions},
};

const COLUMN_WIDTHS: &[f32] = &[75.0, 150.0, 100.0];

pub trait DataWidget {
    fn render_value(&mut self, ui: &mut egui::Ui, types: &Types, state: &mut State);

    fn render_compound(&mut self, ui: &mut egui::Ui, types: &Types, state: &mut State);

    fn is_open(&self, _ui: &mut egui::Ui) -> bool {
        false
    }
}

impl<'a> TypeInstance<'a> {
    pub fn into_data_widget(self, ui: &mut egui::Ui, types: &'a Types) -> Box<dyn DataWidget + 'a> {
        match self.ty() {
            type_crawler::TypeKind::USize { .. } => Box::new(IntegerWidget::new(ui, self)),
            type_crawler::TypeKind::SSize { .. } => Box::new(IntegerWidget::new(ui, self)),
            type_crawler::TypeKind::U64 => Box::new(IntegerWidget::new(ui, self)),
            type_crawler::TypeKind::U32 => Box::new(IntegerWidget::new(ui, self)),
            type_crawler::TypeKind::U16 => Box::new(IntegerWidget::new(ui, self)),
            type_crawler::TypeKind::U8 => Box::new(IntegerWidget::new(ui, self)),
            type_crawler::TypeKind::S64 => Box::new(IntegerWidget::new(ui, self)),
            type_crawler::TypeKind::S32 => Box::new(IntegerWidget::new(ui, self)),
            type_crawler::TypeKind::S16 => Box::new(IntegerWidget::new(ui, self)),
            type_crawler::TypeKind::S8 => Box::new(IntegerWidget::new(ui, self)),
            type_crawler::TypeKind::F32 => Box::new(FloatWidget::new(ui, self)),
            type_crawler::TypeKind::F64 => Box::new(FloatWidget::new(ui, self)),
            type_crawler::TypeKind::LongDouble { .. } => {
                Box::new(WipWidget { data_type: "long double" })
            }
            type_crawler::TypeKind::Char16 => Box::new(WipWidget { data_type: "char16" }),
            type_crawler::TypeKind::Char32 => Box::new(WipWidget { data_type: "char32" }),
            type_crawler::TypeKind::WChar { .. } => Box::new(WipWidget { data_type: "wchar" }),
            type_crawler::TypeKind::Bool => Box::new(BoolWidget { instance: self }),
            type_crawler::TypeKind::Void => Box::new(VoidWidget),
            type_crawler::TypeKind::Reference { referenced_type: pointee_type, .. }
            | type_crawler::TypeKind::Pointer { pointee_type, .. }
            | type_crawler::TypeKind::MemberPointer { pointee_type, .. } => {
                let address = u32::from_le_bytes(self.data()[..].try_into().unwrap_or([0; 4]));
                Box::new(PointerWidget::new(ui, pointee_type, address))
            }
            type_crawler::TypeKind::Array { element_type, size: Some(size) } => {
                Box::new(ArrayWidget::new(ui, element_type, *size, self))
            }
            type_crawler::TypeKind::Array { element_type, size: None } => {
                Box::new(PointerWidget::new(ui, element_type, self.address()))
            }
            type_crawler::TypeKind::Function { .. } => Box::new(IntegerWidget::new(ui, self)),
            type_crawler::TypeKind::Struct(struct_decl) => {
                Box::new(StructWidget::new(ui, struct_decl, self))
            }
            type_crawler::TypeKind::Class(class_decl) => {
                Box::new(StructWidget::new(ui, class_decl, self))
            }
            type_crawler::TypeKind::Union(union_decl) => {
                Box::new(UnionWidget::new(ui, union_decl, self))
            }
            type_crawler::TypeKind::Enum(enum_decl) => {
                Box::new(EnumWidget { enum_decl, instance: self })
            }
            type_crawler::TypeKind::Typedef(typedef) => {
                self.with_type(typedef.underlying_type()).into_data_widget(ui, types)
            }
            type_crawler::TypeKind::Named(name) => match name.as_str() {
                "q20" => Box::new(Fx32Widget::new(ui, self)),
                _ => {
                    if let Some(type_decl) = types.get(name) {
                        self.with_type(type_decl).into_data_widget(ui, types)
                    } else {
                        Box::new(NotFoundWidget { name: name.clone() })
                    }
                }
            },
        }
    }
}

struct VoidWidget;

impl DataWidget for VoidWidget {
    fn render_value(&mut self, _ui: &mut egui::Ui, _types: &Types, _state: &mut State) {}

    fn render_compound(&mut self, _ui: &mut egui::Ui, _types: &Types, _state: &mut State) {}
}

struct IntegerWidget<'a> {
    instance: TypeInstance<'a>,
    show_hex_id: egui::Id,
    text_id: egui::Id,
}

impl<'a> IntegerWidget<'a> {
    fn new(ui: &mut egui::Ui, instance: TypeInstance<'a>) -> Self {
        let show_hex_id = ui.make_persistent_id("show_hex");
        let text_id = ui.make_persistent_id("value");
        Self { instance, show_hex_id, text_id }
    }
}

impl<'a> DataWidget for IntegerWidget<'a> {
    fn render_value(&mut self, ui: &mut egui::Ui, types: &Types, state: &mut State) {
        ui.horizontal(|ui| {
            let mut show_hex =
                ui.ctx().data_mut(|data| data.get_temp::<bool>(self.show_hex_id).unwrap_or(false));
            let mut text =
                ui.ctx().data_mut(|data| data.get_temp::<String>(self.text_id).unwrap_or_default());

            let text_edit =
                egui::TextEdit::singleline(&mut text).desired_width(70.0).show(ui).response;

            if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let value = if let Some(hex_text) = text.strip_prefix("0x") {
                    u32::from_str_radix(hex_text, 16).unwrap_or(0)
                } else {
                    text.parse::<u32>().unwrap_or(0)
                };
                self.instance.write(state, value.to_le_bytes().to_vec());
            }

            if !text_edit.has_focus() {
                let value = self.instance.as_int::<i64>(types).unwrap();
                text = if show_hex {
                    match self.instance.ty().size(types) {
                        1 => format!("{:#x}", value as u8),
                        2 => format!("{:#x}", value as u16),
                        4 => format!("{:#x}", value as u32),
                        8 => format!("{:#x}", value as u64),
                        _ => format!("{:#x}", value),
                    }
                } else {
                    value.to_string()
                };
            }
            ui.ctx().data_mut(|data| data.insert_temp(self.text_id, text));

            if ui.selectable_label(show_hex, "0x").clicked() {
                show_hex = !show_hex;
                ui.ctx().data_mut(|data| data.insert_temp(self.show_hex_id, show_hex));
            }
        });
    }

    fn render_compound(&mut self, ui: &mut egui::Ui, types: &Types, state: &mut State) {
        ui.indent("integer_compound", |ui| {
            columns::fixed_columns(ui, COLUMN_WIDTHS, |columns| {
                ValueBadge::new(types, self.instance.ty()).render(&mut columns[0]);
                columns[1].label("Value");
                self.render_value(&mut columns[2], types, state);
            });
        });
    }
}

struct FloatWidget<'a> {
    instance: TypeInstance<'a>,
    show_hex_id: egui::Id,
    text_id: egui::Id,
}

impl<'a> FloatWidget<'a> {
    fn new(ui: &mut egui::Ui, instance: TypeInstance<'a>) -> Self {
        let show_hex_id = ui.make_persistent_id("show_hex");
        let text_id = ui.make_persistent_id("value");
        Self { instance, show_hex_id, text_id }
    }
}

impl<'a> DataWidget for FloatWidget<'a> {
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types, state: &mut State) {
        ui.horizontal(|ui| {
            let mut show_hex =
                ui.ctx().data_mut(|data| data.get_temp::<bool>(self.show_hex_id).unwrap_or(false));
            let mut text =
                ui.ctx().data_mut(|data| data.get_temp::<String>(self.text_id).unwrap_or_default());

            let text_edit =
                egui::TextEdit::singleline(&mut text).desired_width(70.0).show(ui).response;

            if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let value = if let Some(hex_text) = text.strip_prefix("0x") {
                    let raw_value = u32::from_str_radix(hex_text, 16).unwrap_or(0);
                    f32::from_le_bytes(raw_value.to_le_bytes())
                } else {
                    text.parse::<f32>().unwrap_or(0.0)
                };
                self.instance.write(state, value.to_le_bytes().to_vec());
            }
            if !text_edit.has_focus() {
                let value =
                    u32::from_le_bytes(self.instance.data()[..].try_into().unwrap_or([0; 4]));
                text = if show_hex {
                    format!("{:#x}", value)
                } else {
                    let float = f32::from_le_bytes(value.to_le_bytes());
                    format!("{:.5}", float)
                };
            }
            ui.ctx().data_mut(|data| data.insert_temp(self.text_id, text));

            if ui.selectable_label(show_hex, "0x").clicked() {
                show_hex = !show_hex;
                ui.ctx().data_mut(|data| data.insert_temp(self.show_hex_id, show_hex));
            }
        });
    }

    fn render_compound(&mut self, ui: &mut egui::Ui, types: &Types, state: &mut State) {
        ui.indent("float_compound", |ui| {
            columns::fixed_columns(ui, COLUMN_WIDTHS, |columns| {
                ValueBadge::new(types, self.instance.ty()).render(&mut columns[0]);
                columns[1].label("Value");
                self.render_value(&mut columns[2], types, state);
            });
        });
    }
}

struct BoolWidget<'a> {
    instance: TypeInstance<'a>,
}

impl<'a> DataWidget for BoolWidget<'a> {
    fn render_value(&mut self, ui: &mut egui::Ui, types: &Types, state: &mut State) {
        let value = self.instance.as_int::<u8>(types).unwrap_or(0);

        let mut checked = value != 0;
        let text: Cow<str> = if value > 1 {
            format!("(0x{:02x})", value).into()
        } else {
            "".into()
        };
        if ui.checkbox(&mut checked, text).changed() {
            self.instance.write(state, if checked { vec![1] } else { vec![0] });
        }
    }

    fn render_compound(&mut self, ui: &mut egui::Ui, types: &Types, state: &mut State) {
        ui.indent("bool_compound", |ui| {
            columns::fixed_columns(ui, COLUMN_WIDTHS, |columns| {
                ValueBadge::new(types, &type_crawler::TypeKind::Bool).render(&mut columns[0]);
                columns[1].label("Value");
                self.render_value(&mut columns[2], types, state);
            });
        });
    }
}

struct ArrayWidget<'a> {
    element_type: &'a type_crawler::TypeKind,
    size: usize,
    instance: TypeInstance<'a>,
    open_id: egui::Id,
}

impl<'a> ArrayWidget<'a> {
    fn new(
        ui: &mut egui::Ui,
        element_type: &'a type_crawler::TypeKind,
        size: usize,
        instance: TypeInstance<'a>,
    ) -> Self {
        let open_id = ui.make_persistent_id("array_open");
        Self { element_type, size, instance, open_id }
    }
}

impl<'a> DataWidget for ArrayWidget<'a> {
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types, _state: &mut State) {
        let mut open = self.is_open(ui);
        if ui.selectable_label(open, "Open").clicked() {
            open = !open;
            ui.ctx().data_mut(|data| data.insert_temp(self.open_id, open));
        }
    }

    fn render_compound(&mut self, ui: &mut egui::Ui, types: &Types, state: &mut State) {
        ui.indent("array_compound", |ui| {
            let stride = self.element_type.stride(types);
            for i in 0..self.size {
                let offset = i * stride;
                let field_instance = self.instance.slice(types, self.element_type, offset, None);

                ui.push_id(i, |ui| {
                    let mut widget = field_instance.into_data_widget(ui, types);
                    columns::fixed_columns(ui, COLUMN_WIDTHS, |columns| {
                        ValueBadge::new(types, self.element_type).render(&mut columns[0]);
                        columns[1].label(format!("[{i}]"));
                        widget.render_value(&mut columns[2], types, state);
                    });
                    if widget.is_open(ui) {
                        widget.render_compound(ui, types, state);
                    }
                });
            }
        });
    }

    fn is_open(&self, ui: &mut egui::Ui) -> bool {
        ui.ctx().data_mut(|data| data.get_temp::<bool>(self.open_id).unwrap_or(false))
    }
}

struct PointerWidget<'a> {
    pointee_type: &'a type_crawler::TypeKind,
    address: u32,
    list_length_id: egui::Id,
    open_id: egui::Id,
}

impl<'a> PointerWidget<'a> {
    fn new(ui: &mut egui::Ui, pointee_type: &'a type_crawler::TypeKind, address: u32) -> Self {
        let list_length_id = ui.make_persistent_id("pointer_list_length");
        let open_id = ui.make_persistent_id("pointer_open");
        Self { pointee_type, address, list_length_id, open_id }
    }
}

impl DataWidget for PointerWidget<'_> {
    fn render_value(&mut self, ui: &mut egui::Ui, types: &Types, _state: &mut State) {
        if self.pointee_type.size(types) == 0 {
            let mut str = format!("{:#010x}", self.address);
            egui::TextEdit::singleline(&mut str).desired_width(70.0).show(ui);
            return;
        }
        if self.address == 0 {
            ui.label("NULL");
            ui.ctx().data_mut(|data| data.insert_temp(self.open_id, false));
            return;
        }
        ui.horizontal(|ui| {
            let mut open = self.is_open(ui);
            let open_label = ui.selectable_label(open, "Open");
            if open_label.clicked() {
                open = !open;
                ui.ctx().data_mut(|data| data.insert_temp(self.open_id, open));
            }
            if open_label.hovered() {
                egui::Tooltip::for_widget(&open_label).at_pointer().gap(12.0).show(|ui| {
                    ui.label(format!("{:#x}", self.address));
                });
            }

            let mut list_length =
                ui.ctx().data_mut(|data| data.get_temp::<usize>(self.list_length_id).unwrap_or(1));
            if egui::DragValue::new(&mut list_length).ui(ui).changed() {
                ui.ctx().data_mut(|data| data.insert_temp(self.list_length_id, list_length));
            }
        });
    }

    fn render_compound(&mut self, ui: &mut egui::Ui, types: &Types, state: &mut State) {
        let list_length =
            ui.ctx().data_mut(|data| data.get_temp::<usize>(self.list_length_id).unwrap_or(1));
        let stride = self.pointee_type.stride(types);
        if stride == 0 {
            return;
        }
        let size = stride * list_length;
        state.request(self.address, size);
        let Some(data) = state.get_data(self.address).map(|d| d.to_vec()) else {
            ui.label("Pointer data not found");
            return;
        };
        let instance = TypeInstance::new(TypeInstanceOptions {
            ty: self.pointee_type,
            address: self.address,
            bit_field_range: None,
            data: Cow::Owned(data),
        });

        if list_length == 1 {
            instance.into_data_widget(ui, types).render_compound(ui, types, state);
            return;
        }
        ui.indent("pointer_compound", |ui| {
            for i in 0..list_length {
                ui.push_id(i, |ui| {
                    let offset = i * stride;
                    let field_instance = instance.slice(types, self.pointee_type, offset, None);

                    let mut widget = field_instance.into_data_widget(ui, types);
                    columns::fixed_columns(ui, COLUMN_WIDTHS, |columns| {
                        ValueBadge::new(types, self.pointee_type).render(&mut columns[0]);
                        columns[1].label(format!("[{i}]"));
                        widget.render_value(&mut columns[2], types, state);
                    });
                    if widget.is_open(ui) {
                        widget.render_compound(ui, types, state);
                    }
                });
            }
        });
    }

    fn is_open(&self, ui: &mut egui::Ui) -> bool {
        ui.ctx().data_mut(|data| data.get_temp::<bool>(self.open_id).unwrap_or(false))
    }
}

struct WipWidget {
    data_type: &'static str,
}

impl DataWidget for WipWidget {
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types, _state: &mut State) {
        ui.label(
            egui::RichText::new(format!("{} value not implemented", self.data_type))
                .color(egui::Color32::RED),
        );
    }

    fn render_compound(&mut self, ui: &mut egui::Ui, _types: &Types, _state: &mut State) {
        ui.label(
            egui::RichText::new(format!("{} compound not implemented", self.data_type))
                .color(egui::Color32::RED),
        );
    }
}

struct NotFoundWidget {
    name: String,
}

impl DataWidget for NotFoundWidget {
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types, _state: &mut State) {
        ui.label(
            egui::RichText::new(format!("Type '{}' not found", self.name))
                .color(egui::Color32::RED),
        );
    }

    fn render_compound(&mut self, _ui: &mut egui::Ui, _types: &Types, _state: &mut State) {}
}

struct Fx32Widget<'a> {
    instance: TypeInstance<'a>,
    show_hex_id: egui::Id,
    text_id: egui::Id,
}

impl<'a> Fx32Widget<'a> {
    fn new(ui: &mut egui::Ui, instance: TypeInstance<'a>) -> Self {
        let show_hex_id = ui.make_persistent_id("show_hex");
        let text_id = ui.make_persistent_id("text");
        Self { instance, show_hex_id, text_id }
    }
}

impl<'a> DataWidget for Fx32Widget<'a> {
    fn render_value(&mut self, ui: &mut egui::Ui, types: &Types, state: &mut State) {
        ui.horizontal(|ui| {
            let mut show_hex =
                ui.ctx().data_mut(|data| data.get_temp::<bool>(self.show_hex_id).unwrap_or(false));
            let mut text =
                ui.ctx().data_mut(|data| data.get_temp::<String>(self.text_id).unwrap_or_default());

            let text_edit =
                egui::TextEdit::singleline(&mut text).desired_width(70.0).show(ui).response;

            if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let value = if let Some(hex_text) = text.strip_prefix("0x") {
                    i32::from_str_radix(hex_text, 16).unwrap_or(0)
                } else {
                    (text.parse::<f32>().unwrap_or(0.0) * 4096.0) as i32
                };
                self.instance.write(state, value.to_le_bytes().to_vec());
            }
            if !text_edit.has_focus() {
                let value = self.instance.as_int::<i32>(types).unwrap();
                text = if show_hex {
                    format!("{:#x}", value)
                } else {
                    let q20 = value as f32 / 4096.0;
                    format!("{:.5}", q20)
                };
            }
            ui.ctx().data_mut(|data| data.insert_temp(self.text_id, text));

            if ui.selectable_label(show_hex, "0x").clicked() {
                show_hex = !show_hex;
                ui.ctx().data_mut(|data| data.insert_temp(self.show_hex_id, show_hex));
            }
        });
    }

    fn render_compound(&mut self, ui: &mut egui::Ui, types: &Types, state: &mut State) {
        ui.indent("fx32_compound", |ui| {
            columns::fixed_columns(ui, COLUMN_WIDTHS, |columns| {
                ValueBadge::new(types, &type_crawler::TypeKind::Named("q20".to_string()))
                    .render(&mut columns[0]);
                columns[1].label("Value");
                self.render_value(&mut columns[2], types, state);
            });
        });
    }
}

struct EnumWidget<'a> {
    enum_decl: &'a type_crawler::EnumDecl,
    instance: TypeInstance<'a>,
}

impl<'a> DataWidget for EnumWidget<'a> {
    fn render_value(&mut self, ui: &mut egui::Ui, types: &Types, state: &mut State) {
        let size = self.enum_decl.size();
        let mut value = self.instance.as_int::<i64>(types).unwrap();

        let current_constant = self.enum_decl.get_by_value(value);
        let selected_text: Cow<str> = if let Some(constant) = current_constant {
            constant.name().into()
        } else {
            format!("{:#x}", value).into()
        };

        egui::ComboBox::new("enum_value", "").selected_text(selected_text).show_ui(ui, |ui| {
            for constant in self.enum_decl.constants() {
                if ui.selectable_value(&mut value, constant.value(), constant.name()).clicked() {
                    let constant_bytes = match size {
                        1 => (constant.value() as u8).to_le_bytes().to_vec(),
                        2 => (constant.value() as u16).to_le_bytes().to_vec(),
                        4 => (constant.value() as u32).to_le_bytes().to_vec(),
                        8 => (constant.value() as u64).to_le_bytes().to_vec(),
                        _ => panic!("Unsupported enum size"),
                    };
                    self.instance.write(state, constant_bytes);
                }
            }
        });
    }

    fn render_compound(&mut self, ui: &mut egui::Ui, types: &Types, state: &mut State) {
        ui.indent("enum_compound", |ui| {
            columns::fixed_columns(ui, COLUMN_WIDTHS, |columns| {
                ValueBadge::new_enum(self.enum_decl).render(&mut columns[0]);
                columns[1].label("Value");
                self.render_value(&mut columns[2], types, state);
            });
        });
    }
}

struct StructWidget<'a> {
    struct_decl: &'a type_crawler::StructDecl,
    instance: TypeInstance<'a>,
    open_id: egui::Id,
}

impl<'a> StructWidget<'a> {
    fn new(
        ui: &mut egui::Ui,
        struct_decl: &'a type_crawler::StructDecl,
        instance: TypeInstance<'a>,
    ) -> Self {
        let open_id = ui.make_persistent_id("struct_open");
        Self { struct_decl, instance, open_id }
    }

    fn render_fields(&self, ui: &mut egui::Ui, types: &type_crawler::Types, state: &mut State) {
        let fields = self.struct_decl.fields();
        if fields.is_empty() {
            return;
        }
        ui.heading(self.struct_decl.name().unwrap_or("Unnamed Struct"));
        for field in fields {
            let offset = field.offset_bytes();
            let bit_field_range = if let Some(width) = field.bit_field_width() {
                let start = (field.offset_bits() - offset * 8) as u8;
                Some(start..start + width)
            } else {
                None
            };
            let field_instance = self.instance.slice(types, field.kind(), offset, bit_field_range);

            ui.push_id(offset, |ui| {
                let mut widget = field_instance.into_data_widget(ui, types);
                columns::fixed_columns(ui, COLUMN_WIDTHS, |columns| {
                    ValueBadge::new(types, field.kind()).render(&mut columns[0]);
                    columns[1].label(field.name().unwrap_or(""));
                    widget.render_value(&mut columns[2], types, state);
                });
                if widget.is_open(ui) {
                    widget.render_compound(ui, types, state);
                }
            });
        }
    }

    fn render_base_types_and_fields(&self, ui: &mut egui::Ui, types: &'a Types, state: &mut State) {
        for base_type in self.struct_decl.base_types() {
            let Some(base_struct) = types.get(base_type).and_then(|ty| ty.as_struct(types)) else {
                ui.label(format!("Base type '{base_type}' not found"));
                continue;
            };
            Self {
                struct_decl: base_struct,
                instance: self.instance.clone(),
                open_id: self.open_id,
            }
            .render_base_types_and_fields(ui, types, state);
        }
        self.render_fields(ui, types, state);
    }
}

impl<'a> DataWidget for StructWidget<'a> {
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types, _state: &mut State) {
        let mut open = self.is_open(ui);
        if ui.selectable_label(open, "Open").clicked() {
            open = !open;
            ui.ctx().data_mut(|data| data.insert_temp(self.open_id, open));
        }
    }

    fn render_compound(&mut self, ui: &mut egui::Ui, types: &Types, state: &mut State) {
        ui.indent("struct_compound", |ui| {
            self.render_base_types_and_fields(ui, types, state);
        });
    }

    fn is_open(&self, ui: &mut egui::Ui) -> bool {
        ui.ctx().data_mut(|data| data.get_temp::<bool>(self.open_id).unwrap_or(false))
    }
}

struct UnionWidget<'a> {
    union_decl: &'a type_crawler::UnionDecl,
    instance: TypeInstance<'a>,
    open_id: egui::Id,
}

impl<'a> UnionWidget<'a> {
    fn new(
        ui: &mut egui::Ui,
        union_decl: &'a type_crawler::UnionDecl,
        instance: TypeInstance<'a>,
    ) -> Self {
        let open_id = ui.make_persistent_id("union_open");
        Self { union_decl, instance, open_id }
    }
}

impl<'a> DataWidget for UnionWidget<'a> {
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types, _state: &mut State) {
        let mut open = self.is_open(ui);
        if ui.selectable_label(open, "Open").clicked() {
            open = !open;
            ui.ctx().data_mut(|data| data.insert_temp(self.open_id, open));
        }
    }

    fn render_compound(&mut self, ui: &mut egui::Ui, types: &Types, state: &mut State) {
        ui.indent("union_compound", |ui| {
            for (i, field) in self.union_decl.fields().iter().enumerate() {
                let bit_field_range = field.bit_field_width().map(|width| 0..width);
                let field_instance = self.instance.slice(types, field.kind(), 0, bit_field_range);

                ui.push_id(i, |ui| {
                    let mut widget = field_instance.into_data_widget(ui, types);
                    columns::fixed_columns(ui, COLUMN_WIDTHS, |columns| {
                        ValueBadge::new(types, field.kind()).render(&mut columns[0]);
                        columns[1].label(field.name().unwrap_or(""));
                        widget.render_value(&mut columns[2], types, state);
                    });
                    if widget.is_open(ui) {
                        widget.render_compound(ui, types, state);
                    }
                });
            }
        });
    }

    fn is_open(&self, ui: &mut egui::Ui) -> bool {
        ui.ctx().data_mut(|data| data.get_temp::<bool>(self.open_id).unwrap_or(false))
    }
}

struct ValueBadge<'a> {
    text: Cow<'a, str>,
    tooltip: Option<String>,
    background: &'static str,
    color: &'static str,
}

impl<'a> ValueBadge<'a> {
    fn render(self, ui: &mut egui::Ui) {
        let label = ui.label(
            egui::RichText::new(self.text)
                .background_color(egui::Color32::from_hex(self.background).unwrap())
                .color(egui::Color32::from_hex(self.color).unwrap()),
        );
        if label.hovered()
            && let Some(tooltip) = self.tooltip
        {
            egui::Tooltip::for_widget(&label).at_pointer().gap(12.0).show(|ui| {
                ui.label(tooltip);
            });
        }
    }
    fn new(types: &'a Types, kind: &'a type_crawler::TypeKind) -> Self {
        match kind {
            type_crawler::TypeKind::USize { .. } => ValueBadge {
                text: "usize".into(),
                tooltip: None,
                background: "#224eff",
                color: "#ffffff",
            },
            type_crawler::TypeKind::SSize { .. } => ValueBadge {
                text: "ssize".into(),
                tooltip: None,
                background: "#ff4e22",
                color: "#ffffff",
            },
            type_crawler::TypeKind::U64 => ValueBadge {
                text: "u64".into(),
                tooltip: None,
                background: "#0033ff",
                color: "#ffffff",
            },
            type_crawler::TypeKind::U32 => ValueBadge {
                text: "u32".into(),
                tooltip: None,
                background: "#466bff",
                color: "#ffffff",
            },
            type_crawler::TypeKind::U16 => ValueBadge {
                text: "u16".into(),
                tooltip: None,
                background: "#7691ff",
                color: "#ffffff",
            },
            type_crawler::TypeKind::U8 => ValueBadge {
                text: "u8".into(),
                tooltip: None,
                background: "#a9baff",
                color: "#000000",
            },
            type_crawler::TypeKind::S64 => ValueBadge {
                text: "s64".into(),
                tooltip: None,
                background: "#ff3300",
                color: "#ffffff",
            },
            type_crawler::TypeKind::S32 => ValueBadge {
                text: "s32".into(),
                tooltip: None,
                background: "#ff6b46",
                color: "#000000",
            },
            type_crawler::TypeKind::S16 => ValueBadge {
                text: "s16".into(),
                tooltip: None,
                background: "#ff9176",
                color: "#000000",
            },
            type_crawler::TypeKind::S8 => ValueBadge {
                text: "s8".into(),
                tooltip: None,
                background: "#ffbaa9",
                color: "#000000",
            },
            type_crawler::TypeKind::F32 => ValueBadge {
                text: "f32".into(),
                tooltip: None,
                background: "#00ffee",
                color: "#000000",
            },
            type_crawler::TypeKind::F64 => ValueBadge {
                text: "f64".into(),
                tooltip: None,
                background: "#00b0a5",
                color: "#000000",
            },
            type_crawler::TypeKind::LongDouble { .. } => ValueBadge {
                text: "long double".into(),
                tooltip: None,
                background: "rgba(0, 126, 126, 1)",
                color: "#ffffff",
            },
            type_crawler::TypeKind::Char16 => ValueBadge {
                text: "char16".into(),
                tooltip: None,
                background: "#ff9176",
                color: "#000000",
            },
            type_crawler::TypeKind::Char32 => ValueBadge {
                text: "char32".into(),
                tooltip: None,
                background: "#ff6b46",
                color: "#000000",
            },
            type_crawler::TypeKind::WChar { .. } => ValueBadge {
                text: "wchar".into(),
                tooltip: None,
                background: "#ff4e22",
                color: "#ffffff",
            },
            type_crawler::TypeKind::Bool => ValueBadge {
                text: "bool".into(),
                tooltip: None,
                background: "#008d00",
                color: "#ffffff",
            },
            type_crawler::TypeKind::Void => ValueBadge {
                text: "void".into(),
                tooltip: None,
                background: "#242424",
                color: "#ffffff",
            },
            type_crawler::TypeKind::Reference { referenced_type: pointee_type, .. } => {
                let ValueBadge { text, tooltip, background, color } =
                    Self::new(types, pointee_type);
                let text = tooltip.as_deref().unwrap_or(&text);
                let (new_text, tooltip) = if text.len() <= 10 {
                    (format!("{text}&").into(), None)
                } else {
                    ("pointer".into(), Some(format!("{text}&")))
                };
                ValueBadge { text: new_text, tooltip, background, color }
            }
            type_crawler::TypeKind::Pointer { pointee_type, .. } => {
                let ValueBadge { text, tooltip, background, color } =
                    Self::new(types, pointee_type);
                let text = tooltip.as_deref().unwrap_or(&text);
                let (new_text, tooltip) = if text.len() <= 10 {
                    (format!("{text}*").into(), None)
                } else {
                    ("pointer".into(), Some(format!("{text}*")))
                };
                ValueBadge { text: new_text, tooltip, background, color }
            }
            type_crawler::TypeKind::MemberPointer { pointee_type, record_name, .. } => {
                let ValueBadge { text, tooltip, background, color } =
                    Self::new(types, pointee_type);
                let text = tooltip.as_deref().unwrap_or(&text);
                let (new_text, tooltip) = if text.len() <= 10 {
                    (format!("{text}*").into(), None)
                } else {
                    ("pointer".into(), Some(format!("{text} {record_name}::*")))
                };
                ValueBadge { text: new_text, tooltip, background, color }
            }
            type_crawler::TypeKind::Array { element_type, .. } => {
                let ValueBadge { text, tooltip, background, color } =
                    Self::new(types, element_type);
                let text = tooltip.as_deref().unwrap_or(&text);
                let (new_text, tooltip) = if text.len() <= 10 {
                    (format!("{text}[]").into(), None)
                } else {
                    ("array".into(), Some(format!("{text}[]")))
                };
                ValueBadge { text: new_text, tooltip, background, color }
            }
            type_crawler::TypeKind::Function { .. } => ValueBadge {
                text: "fn".into(),
                tooltip: None,
                background: "#35620bff",
                color: "#ffffff",
            },
            type_crawler::TypeKind::Struct(struct_decl) => Self::new_struct(struct_decl),
            type_crawler::TypeKind::Class(class_decl) => Self::new_class(class_decl),
            type_crawler::TypeKind::Union(union_decl) => Self::new_union(union_decl),
            type_crawler::TypeKind::Enum(enum_decl) => Self::new_enum(enum_decl),
            type_crawler::TypeKind::Typedef(typedef) => Self::new(types, typedef.underlying_type()),
            type_crawler::TypeKind::Named(name) => match name.as_str() {
                "q20" => ValueBadge {
                    text: "q20".into(),
                    tooltip: None,
                    background: "#006abb",
                    color: "#ffffff",
                },
                _ => {
                    let Some(ty) = types.get(name) else {
                        return ValueBadge {
                            text: "unknown".into(),
                            tooltip: None,
                            background: "#000000ff",
                            color: "#ffffff",
                        };
                    };
                    Self::new(types, ty)
                }
            },
        }
    }

    fn new_struct(struct_decl: &'a type_crawler::StructDecl) -> Self {
        let full_name = struct_decl.name();
        let (text, tooltip) = if let Some(name) = full_name
            && name.len() <= 10
        {
            (name.into(), None)
        } else {
            ("struct".into(), full_name.map(|n| n.to_string()))
        };
        ValueBadge { text, tooltip, background: "#af1cc9", color: "#ffffff" }
    }

    fn new_class(struct_decl: &'a type_crawler::StructDecl) -> Self {
        let full_name = struct_decl.name();
        let (text, tooltip) = if let Some(name) = full_name
            && name.len() <= 10
        {
            (name.into(), None)
        } else {
            ("class".into(), full_name.map(|n| n.to_string()))
        };
        ValueBadge { text, tooltip, background: "#af1cc9", color: "#ffffff" }
    }

    fn new_union(union_decl: &'a type_crawler::UnionDecl) -> Self {
        let full_name = union_decl.name();
        let (text, tooltip) = if let Some(name) = full_name
            && name.len() <= 10
        {
            (name.into(), None)
        } else {
            ("union".into(), full_name.map(|n| n.to_string()))
        };
        ValueBadge { text, tooltip, background: "#c9bb1c", color: "#000000" }
    }

    fn new_enum(enum_decl: &'a type_crawler::EnumDecl) -> Self {
        let full_name = enum_decl.name();
        let (text, tooltip) = if let Some(name) = full_name
            && name.len() <= 10
        {
            (name.into(), None)
        } else {
            ("enum".into(), full_name.map(|n| n.to_string()))
        };
        ValueBadge { text, tooltip, background: "#ff8c00", color: "#ffffff" }
    }
}
