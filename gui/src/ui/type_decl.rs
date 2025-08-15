use std::borrow::Cow;

use dzv_core::state::State;
use eframe::egui::{self, Widget};
use type_crawler::Types;

use crate::ui::columns;

const COLUMN_WIDTHS: &[f32] = &[75.0, 150.0, 100.0];

pub trait DataWidget {
    fn render_value(&mut self, ui: &mut egui::Ui, types: &Types, state: &mut State);

    fn render_compound(&self, ui: &mut egui::Ui, types: &Types, state: &mut State);

    fn is_open(&self, _ui: &mut egui::Ui) -> bool {
        false
    }
}

pub trait AsDataWidget {
    fn as_data_widget<'a>(
        &self,
        ui: &mut egui::Ui,
        types: &Types,
        instance: TypeInstance<'a>,
    ) -> Box<dyn DataWidget + 'a>;
}

#[derive(Clone, Copy)]
pub struct TypeInstance<'a> {
    data: &'a [u8],
}

impl<'a> TypeInstance<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    fn slice(&self, offset: usize, size: usize) -> Self {
        let start = offset.min(self.data.len());
        let end = (offset + size).min(self.data.len());
        Self::new(&self.data[start..end])
    }

    fn get(&self, size: usize) -> &[u8] {
        let end = size.min(self.data.len());
        &self.data[..end]
    }
}

impl AsDataWidget for type_crawler::TypeKind {
    fn as_data_widget<'a>(
        &self,
        ui: &mut egui::Ui,
        types: &Types,
        instance: TypeInstance<'a>,
    ) -> Box<dyn DataWidget + 'a> {
        match self {
            type_crawler::TypeKind::USize { .. } => {
                let value = u32::from_le_bytes(instance.get(4).try_into().unwrap_or([0; 4]));
                Box::new(IntegerWidget::new(ui, value))
            }
            type_crawler::TypeKind::SSize { .. } => {
                let value = i32::from_le_bytes(instance.get(4).try_into().unwrap_or([0; 4]));
                Box::new(IntegerWidget::new(ui, value))
            }
            type_crawler::TypeKind::U64 => {
                let value = u64::from_le_bytes(instance.get(8).try_into().unwrap_or([0; 8]));
                Box::new(IntegerWidget::new(ui, value))
            }
            type_crawler::TypeKind::U32 => {
                let value = u32::from_le_bytes(instance.get(4).try_into().unwrap_or([0; 4]));
                Box::new(IntegerWidget::new(ui, value))
            }
            type_crawler::TypeKind::U16 => {
                let value = u16::from_le_bytes(instance.get(2).try_into().unwrap_or([0; 2]));
                Box::new(IntegerWidget::new(ui, value))
            }
            type_crawler::TypeKind::U8 => {
                let value = instance.get(1).first().copied().unwrap_or(0);
                Box::new(IntegerWidget::new(ui, value))
            }
            type_crawler::TypeKind::S64 => {
                let value = i64::from_le_bytes(instance.get(8).try_into().unwrap_or([0; 8]));
                Box::new(IntegerWidget::new(ui, value))
            }
            type_crawler::TypeKind::S32 => {
                let value = i32::from_le_bytes(instance.get(4).try_into().unwrap_or([0; 4]));
                Box::new(IntegerWidget::new(ui, value))
            }
            type_crawler::TypeKind::S16 => {
                let value = i16::from_le_bytes(instance.get(2).try_into().unwrap_or([0; 2]));
                Box::new(IntegerWidget::new(ui, value))
            }
            type_crawler::TypeKind::S8 => {
                let value = instance.get(1).first().copied().unwrap_or(0) as i8;
                Box::new(IntegerWidget::new(ui, value))
            }
            type_crawler::TypeKind::Bool => {
                let value = instance.get(1).first().copied().unwrap_or(0);
                Box::new(BoolWidget { value })
            }
            type_crawler::TypeKind::Void => Box::new(VoidWidget),
            type_crawler::TypeKind::Pointer { pointee_type, .. } => {
                let address = u32::from_le_bytes(instance.get(4).try_into().unwrap_or([0; 4]));
                Box::new(PointerWidget::new(ui, *pointee_type.clone(), address))
            }
            type_crawler::TypeKind::Array { element_type, size: Some(size) } => {
                Box::new(ArrayWidget::new(ui, *element_type.clone(), *size, instance))
            }
            type_crawler::TypeKind::Array { size: None, .. } => {
                Box::new(WipWidget { data_type: "incomplete array" })
            }
            type_crawler::TypeKind::Function { .. } => {
                let value = u32::from_le_bytes(instance.get(4).try_into().unwrap_or([0; 4]));
                Box::new(IntegerWidget::new(ui, value))
            }
            type_crawler::TypeKind::Struct(struct_decl) => {
                struct_decl.as_data_widget(ui, types, instance)
            }
            type_crawler::TypeKind::Union(union_decl) => {
                union_decl.as_data_widget(ui, types, instance)
            }
            type_crawler::TypeKind::Named(name) => match name.as_str() {
                "q20" => {
                    let value = i32::from_le_bytes(instance.get(4).try_into().unwrap_or([0; 4]));
                    Box::new(Fx32Widget::new(ui, value))
                }
                _ => {
                    if let Some(type_decl) = types.get(name) {
                        type_decl.as_data_widget(ui, types, instance)
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

    fn render_compound(&self, _ui: &mut egui::Ui, _types: &Types, _state: &mut State) {}
}

struct IntegerWidget<T> {
    value: T,
    show_hex_id: egui::Id,
}

impl<T> IntegerWidget<T> {
    fn new(ui: &mut egui::Ui, value: T) -> Self {
        let show_hex_id = ui.make_persistent_id("show_hex");
        Self { value, show_hex_id }
    }
}

impl<T> DataWidget for IntegerWidget<T>
where
    T: std::fmt::LowerHex + std::fmt::Display + Copy,
{
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types, _state: &mut State) {
        ui.horizontal(|ui| {
            let mut show_hex =
                ui.ctx().data_mut(|data| data.get_temp::<bool>(self.show_hex_id).unwrap_or(false));
            let mut text = if show_hex {
                format!("{:#x}", self.value)
            } else {
                self.value.to_string()
            };
            egui::TextEdit::singleline(&mut text).desired_width(70.0).show(ui);

            if ui.selectable_label(show_hex, "0x").clicked() {
                show_hex = !show_hex;
                ui.ctx().data_mut(|data| data.insert_temp(self.show_hex_id, show_hex));
            }
        });
    }

    fn render_compound(&self, _ui: &mut egui::Ui, _types: &Types, _state: &mut State) {}
}

struct BoolWidget {
    value: u8,
}

impl DataWidget for BoolWidget {
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types, _state: &mut State) {
        let mut checked = self.value != 0;
        let text: Cow<str> = if self.value > 1 {
            format!("(0x{:02x})", self.value).into()
        } else {
            "".into()
        };
        ui.checkbox(&mut checked, text);
    }

    fn render_compound(&self, _ui: &mut egui::Ui, _types: &Types, _state: &mut State) {}
}

struct ArrayWidget<'a> {
    element_type: type_crawler::TypeKind,
    size: usize,
    instance: TypeInstance<'a>,
    open_id: egui::Id,
}

impl<'a> ArrayWidget<'a> {
    fn new(
        ui: &mut egui::Ui,
        element_type: type_crawler::TypeKind,
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

    fn render_compound(&self, ui: &mut egui::Ui, types: &Types, state: &mut State) {
        ui.indent("array_compound", |ui| {
            let element_size = self.element_type.size(types);
            let stride = self.element_type.stride(types);
            for i in 0..self.size {
                let offset = i * stride;
                let field_instance = self.instance.slice(offset, element_size);

                ui.push_id(i, |ui| {
                    let mut widget = self.element_type.as_data_widget(ui, types, field_instance);
                    columns::fixed_columns(ui, COLUMN_WIDTHS, |columns| {
                        render_value_badge(&mut columns[0], types, &self.element_type);
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

struct PointerWidget {
    pointee_type: type_crawler::TypeKind,
    address: u32,
    list_length_id: egui::Id,
    open_id: egui::Id,
}

impl PointerWidget {
    fn new(ui: &mut egui::Ui, pointee_type: type_crawler::TypeKind, address: u32) -> Self {
        let list_length_id = ui.make_persistent_id("pointer_list_length");
        let open_id = ui.make_persistent_id("pointer_open");
        Self { pointee_type, address, list_length_id, open_id }
    }
}

impl DataWidget for PointerWidget {
    fn render_value(&mut self, ui: &mut egui::Ui, types: &Types, _state: &mut State) {
        if self.pointee_type.size(types) == 0 {
            let mut str = format!("{:#010x}", self.address);
            egui::TextEdit::singleline(&mut str).desired_width(70.0).show(ui);
            return;
        }
        ui.horizontal(|ui| {
            let mut open = self.is_open(ui);
            if ui.selectable_label(open, "Open").clicked() {
                open = !open;
                ui.ctx().data_mut(|data| data.insert_temp(self.open_id, open));
            }

            let mut list_length =
                ui.ctx().data_mut(|data| data.get_temp::<usize>(self.list_length_id).unwrap_or(1));
            if egui::DragValue::new(&mut list_length).ui(ui).changed() {
                ui.ctx().data_mut(|data| data.insert_temp(self.list_length_id, list_length));
            }
        });
    }

    fn render_compound(&self, ui: &mut egui::Ui, types: &Types, state: &mut State) {
        let list_length =
            ui.ctx().data_mut(|data| data.get_temp::<usize>(self.list_length_id).unwrap_or(1));
        let stride = self.pointee_type.stride(types);
        if stride == 0 {
            return;
        }
        let element_size = self.pointee_type.size(types);
        let size = stride * list_length;
        state.request(self.address, size);
        let Some(data) = state.get_data(self.address).map(|d| d.to_vec()) else {
            ui.label("Pointer data not found");
            return;
        };
        let instance = TypeInstance::new(&data);

        if list_length == 1 {
            self.pointee_type.as_data_widget(ui, types, instance).render_compound(ui, types, state);
            return;
        }
        ui.indent("pointer_compound", |ui| {
            for i in 0..list_length {
                ui.push_id(i, |ui| {
                    let offset = i * stride;
                    let field_instance = instance.slice(offset, element_size);

                    let mut widget = self.pointee_type.as_data_widget(ui, types, field_instance);
                    columns::fixed_columns(ui, COLUMN_WIDTHS, |columns| {
                        render_value_badge(&mut columns[0], types, &self.pointee_type);
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

    fn render_compound(&self, ui: &mut egui::Ui, _types: &Types, _state: &mut State) {
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

    fn render_compound(&self, _ui: &mut egui::Ui, _types: &Types, _state: &mut State) {}
}

struct Fx32Widget {
    value: i32,
    show_hex_id: egui::Id,
}

impl Fx32Widget {
    fn new(ui: &mut egui::Ui, value: i32) -> Self {
        let show_hex_id = ui.make_persistent_id("show_hex");
        Self { value, show_hex_id }
    }
}

impl DataWidget for Fx32Widget {
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types, _state: &mut State) {
        ui.horizontal(|ui| {
            let mut show_hex =
                ui.ctx().data_mut(|data| data.get_temp::<bool>(self.show_hex_id).unwrap_or(false));

            let mut text = if show_hex {
                format!("{:#x}", self.value)
            } else {
                let q20 = self.value as f32 / 4096.0;
                format!("{:.5}", q20)
            };
            egui::TextEdit::singleline(&mut text).desired_width(70.0).show(ui);

            if ui.selectable_label(show_hex, "0x").clicked() {
                show_hex = !show_hex;
                ui.ctx().data_mut(|data| data.insert_temp(self.show_hex_id, show_hex));
            }
        });
    }

    fn render_compound(&self, _ui: &mut egui::Ui, _types: &Types, _state: &mut State) {}
}

impl AsDataWidget for type_crawler::TypeDecl {
    fn as_data_widget<'a>(
        &self,
        ui: &mut egui::Ui,
        types: &Types,
        instance: TypeInstance<'a>,
    ) -> Box<dyn DataWidget + 'a> {
        match self {
            type_crawler::TypeDecl::Typedef(typedef) => typedef.as_data_widget(ui, types, instance),
            type_crawler::TypeDecl::Enum(enum_decl) => {
                enum_decl.as_data_widget(ui, types, instance)
            }
            type_crawler::TypeDecl::Struct(struct_decl) => {
                struct_decl.as_data_widget(ui, types, instance)
            }
            type_crawler::TypeDecl::Union(union_decl) => {
                union_decl.as_data_widget(ui, types, instance)
            }
        }
    }
}

impl AsDataWidget for type_crawler::Typedef {
    fn as_data_widget<'a>(
        &self,
        ui: &mut egui::Ui,
        types: &Types,
        instance: TypeInstance<'a>,
    ) -> Box<dyn DataWidget + 'a> {
        self.underlying_type().as_data_widget(ui, types, instance)
    }
}

struct EnumWidget {
    enum_decl: type_crawler::EnumDecl,
    value: i64,
}

impl AsDataWidget for type_crawler::EnumDecl {
    fn as_data_widget(
        &self,
        _ui: &mut egui::Ui,
        _types: &Types,
        instance: TypeInstance,
    ) -> Box<dyn DataWidget> {
        let size = self.size();
        let mut bytes = [0u8; 8];
        bytes[0..size].copy_from_slice(instance.get(size));
        let value = i64::from_le_bytes(bytes);
        Box::new(EnumWidget { enum_decl: self.clone(), value })
    }
}

impl DataWidget for EnumWidget {
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types, _state: &mut State) {
        let current_constant = self.enum_decl.get(self.value);
        let selected_text: Cow<str> = if let Some(constant) = current_constant {
            constant.name().into()
        } else {
            format!("{:#x}", self.value).into()
        };

        egui::ComboBox::new("enum_value", "Select enum value")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                for constant in self.enum_decl.constants() {
                    ui.selectable_value(&mut self.value, constant.value(), constant.name());
                }
            });
    }

    fn render_compound(&self, _ui: &mut egui::Ui, _types: &Types, _state: &mut State) {}
}

struct StructWidget<'a> {
    struct_decl: type_crawler::StructDecl,
    instance: TypeInstance<'a>,
    open_id: egui::Id,
}

impl<'a> StructWidget<'a> {
    fn new(
        ui: &mut egui::Ui,
        struct_decl: type_crawler::StructDecl,
        instance: TypeInstance<'a>,
    ) -> Self {
        let open_id = ui.make_persistent_id("struct_open");
        Self { struct_decl, instance, open_id }
    }
}

impl AsDataWidget for type_crawler::StructDecl {
    fn as_data_widget<'a>(
        &self,
        ui: &mut egui::Ui,
        _types: &Types,
        instance: TypeInstance<'a>,
    ) -> Box<dyn DataWidget + 'a> {
        Box::new(StructWidget::new(ui, self.clone(), instance))
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

    fn render_compound(&self, ui: &mut egui::Ui, types: &Types, state: &mut State) {
        ui.indent("struct_compound", |ui| {
            for field in self.struct_decl.fields() {
                let offset = field.offset_bytes();
                let size = field.kind().size(types);
                let field_instance = self.instance.slice(offset, size);

                ui.push_id(offset, |ui| {
                    let mut widget = field.kind().as_data_widget(ui, types, field_instance);
                    columns::fixed_columns(ui, COLUMN_WIDTHS, |columns| {
                        render_value_badge(&mut columns[0], types, field.kind());
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

struct UnionWidget<'a> {
    union_decl: type_crawler::UnionDecl,
    instance: TypeInstance<'a>,
    open_id: egui::Id,
}

impl<'a> UnionWidget<'a> {
    fn new(
        ui: &mut egui::Ui,
        union_decl: type_crawler::UnionDecl,
        instance: TypeInstance<'a>,
    ) -> Self {
        let open_id = ui.make_persistent_id("union_open");
        Self { union_decl, instance, open_id }
    }
}

impl AsDataWidget for type_crawler::UnionDecl {
    fn as_data_widget<'a>(
        &self,
        ui: &mut egui::Ui,
        _types: &Types,
        instance: TypeInstance<'a>,
    ) -> Box<dyn DataWidget + 'a> {
        Box::new(UnionWidget::new(ui, self.clone(), instance))
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

    fn render_compound(&self, ui: &mut egui::Ui, types: &Types, state: &mut State) {
        ui.indent("union_compound", |ui| {
            for (i, field) in self.union_decl.fields().iter().enumerate() {
                let size = field.kind().size(types);
                let field_instance = self.instance.slice(0, size);

                ui.push_id(i, |ui| {
                    let mut widget = field.kind().as_data_widget(ui, types, field_instance);
                    columns::fixed_columns(ui, COLUMN_WIDTHS, |columns| {
                        render_value_badge(&mut columns[0], types, field.kind());
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

fn render_value_badge(ui: &mut egui::Ui, types: &Types, kind: &type_crawler::TypeKind) {
    let BadgeStyle { text, tooltip, background, color } = value_badge_style(types, kind);
    let label = ui.label(
        egui::RichText::new(text)
            .background_color(egui::Color32::from_hex(background).unwrap())
            .color(egui::Color32::from_hex(color).unwrap()),
    );
    if label.hovered()
        && let Some(tooltip) = tooltip
    {
        egui::Tooltip::for_widget(&label).at_pointer().gap(12.0).show(|ui| {
            ui.label(tooltip);
        });
    }
}

struct BadgeStyle<'a> {
    text: Cow<'a, str>,
    tooltip: Option<String>,
    background: &'static str,
    color: &'static str,
}

fn value_badge_style<'a>(types: &'a Types, kind: &'a type_crawler::TypeKind) -> BadgeStyle<'a> {
    match kind {
        type_crawler::TypeKind::USize { .. } => BadgeStyle {
            text: "usize".into(),
            tooltip: None,
            background: "#224eff",
            color: "#ffffff",
        },
        type_crawler::TypeKind::SSize { .. } => BadgeStyle {
            text: "ssize".into(),
            tooltip: None,
            background: "#ff4e22",
            color: "#ffffff",
        },
        type_crawler::TypeKind::U64 => BadgeStyle {
            text: "u64".into(),
            tooltip: None,
            background: "#0033ff",
            color: "#ffffff",
        },
        type_crawler::TypeKind::U32 => BadgeStyle {
            text: "u32".into(),
            tooltip: None,
            background: "#466bff",
            color: "#ffffff",
        },
        type_crawler::TypeKind::U16 => BadgeStyle {
            text: "u16".into(),
            tooltip: None,
            background: "#7691ff",
            color: "#ffffff",
        },
        type_crawler::TypeKind::U8 => BadgeStyle {
            text: "u8".into(),
            tooltip: None,
            background: "#a9baff",
            color: "#000000",
        },
        type_crawler::TypeKind::S64 => BadgeStyle {
            text: "s64".into(),
            tooltip: None,
            background: "#ff3300",
            color: "#ffffff",
        },
        type_crawler::TypeKind::S32 => BadgeStyle {
            text: "s32".into(),
            tooltip: None,
            background: "#ff6b46",
            color: "#000000",
        },
        type_crawler::TypeKind::S16 => BadgeStyle {
            text: "s16".into(),
            tooltip: None,
            background: "#ff9176",
            color: "#000000",
        },
        type_crawler::TypeKind::S8 => BadgeStyle {
            text: "s8".into(),
            tooltip: None,
            background: "#ffbaa9",
            color: "#000000",
        },
        type_crawler::TypeKind::Bool => BadgeStyle {
            text: "bool".into(),
            tooltip: None,
            background: "#008d00",
            color: "#ffffff",
        },
        type_crawler::TypeKind::Void => BadgeStyle {
            text: "void".into(),
            tooltip: None,
            background: "#242424",
            color: "#ffffff",
        },
        type_crawler::TypeKind::Pointer { pointee_type, .. } => {
            let BadgeStyle { text, tooltip, background, color } =
                value_badge_style(types, pointee_type);
            let text = tooltip.as_deref().unwrap_or(&text);
            let (new_text, tooltip) = if text.len() <= 10 {
                (format!("{text}*").into(), None)
            } else {
                ("pointer".into(), Some(format!("{text}*")))
            };
            BadgeStyle { text: new_text, tooltip, background, color }
        }
        type_crawler::TypeKind::Array { element_type, .. } => {
            let BadgeStyle { text, tooltip, background, color } =
                value_badge_style(types, element_type);
            let text = tooltip.as_deref().unwrap_or(&text);
            let (new_text, tooltip) = if text.len() <= 10 {
                (format!("{text}[]").into(), None)
            } else {
                ("array".into(), Some(format!("{text}[]")))
            };
            BadgeStyle { text: new_text, tooltip, background, color }
        }
        type_crawler::TypeKind::Function { .. } => BadgeStyle {
            text: "fn".into(),
            tooltip: None,
            background: "#35620bff",
            color: "#ffffff",
        },
        type_crawler::TypeKind::Struct(struct_decl) => struct_badge_style(struct_decl),
        type_crawler::TypeKind::Union(union_decl) => union_badge_style(union_decl),
        type_crawler::TypeKind::Named(name) => match name.as_str() {
            "q20" => BadgeStyle {
                text: "q20".into(),
                tooltip: None,
                background: "#006abb",
                color: "#ffffff",
            },
            _ => {
                let Some(type_decl) = types.get(name) else {
                    return BadgeStyle {
                        text: "unknown".into(),
                        tooltip: None,
                        background: "#000000ff",
                        color: "#ffffff",
                    };
                };
                match type_decl {
                    type_crawler::TypeDecl::Typedef(typedef) => {
                        value_badge_style(types, typedef.underlying_type())
                    }
                    type_crawler::TypeDecl::Enum(enum_decl) => enum_badge_style(enum_decl),
                    type_crawler::TypeDecl::Struct(struct_decl) => struct_badge_style(struct_decl),
                    type_crawler::TypeDecl::Union(union_decl) => union_badge_style(union_decl),
                }
            }
        },
    }
}

fn struct_badge_style(struct_decl: &'_ type_crawler::StructDecl) -> BadgeStyle<'_> {
    let full_name = struct_decl.name();
    let (text, tooltip) = if let Some(name) = full_name
        && name.len() <= 10
    {
        (name.into(), None)
    } else {
        ("struct".into(), full_name.map(|n| n.to_string()))
    };
    BadgeStyle { text, tooltip, background: "#af1cc9", color: "#ffffff" }
}

fn union_badge_style(union_decl: &'_ type_crawler::UnionDecl) -> BadgeStyle<'_> {
    let full_name = union_decl.name();
    let (text, tooltip) = if let Some(name) = full_name
        && name.len() <= 10
    {
        (name.into(), None)
    } else {
        ("union".into(), full_name.map(|n| n.to_string()))
    };
    BadgeStyle { text, tooltip, background: "#c9bb1c", color: "#000000" }
}

fn enum_badge_style(enum_decl: &'_ type_crawler::EnumDecl) -> BadgeStyle<'_> {
    let full_name = enum_decl.name();
    let (text, tooltip) = if let Some(name) = full_name
        && name.len() <= 10
    {
        (name.into(), None)
    } else {
        ("enum".into(), full_name.map(|n| n.to_string()))
    };
    BadgeStyle { text, tooltip, background: "#ff8c00", color: "#ffffff" }
}
