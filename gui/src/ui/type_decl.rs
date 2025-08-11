use std::borrow::Cow;

use eframe::egui;
use type_crawler::Types;

use crate::ui::columns;

pub trait DataWidget {
    fn render_value(&mut self, ui: &mut egui::Ui, types: &Types);

    fn render_compound(&self, ui: &mut egui::Ui, types: &Types);

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
            type_crawler::TypeKind::Pointer { .. } => Box::new(WipWidget { data_type: "pointer" }),
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
    fn render_value(&mut self, _ui: &mut egui::Ui, _types: &Types) {}

    fn render_compound(&self, _ui: &mut egui::Ui, _types: &Types) {}
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
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types) {
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

    fn render_compound(&self, _ui: &mut egui::Ui, _types: &Types) {}
}

struct BoolWidget {
    value: u8,
}

impl DataWidget for BoolWidget {
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types) {
        let mut checked = self.value != 0;
        let text: Cow<str> = if self.value > 1 {
            format!("(0x{:02x})", self.value).into()
        } else {
            "".into()
        };
        ui.checkbox(&mut checked, text);
    }

    fn render_compound(&self, _ui: &mut egui::Ui, _types: &Types) {}
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
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types) {
        let mut open = self.is_open(ui);
        if ui.selectable_label(open, "Open").clicked() {
            open = !open;
            ui.ctx().data_mut(|data| data.insert_temp(self.open_id, open));
        }
    }

    fn render_compound(&self, ui: &mut egui::Ui, types: &Types) {
        ui.indent("array_compound", |ui| {
            let element_size = self.element_type.size(types);
            let stride = self.element_type.stride(types);
            for i in 0..self.size {
                let offset = i * stride;
                let field_instance = self.instance.slice(offset, element_size);

                ui.push_id(i, |ui| {
                    let mut widget = self.element_type.as_data_widget(ui, types, field_instance);
                    columns::columns(ui, &[1, 3, 2], |columns| {
                        render_value_badge(&mut columns[0], types, &self.element_type);
                        columns[1].label(format!("[{i}]"));
                        widget.render_value(&mut columns[2], types);
                    });
                    if widget.is_open(ui) {
                        widget.render_compound(ui, types);
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
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types) {
        ui.label(
            egui::RichText::new(format!("{} value not implemented", self.data_type))
                .color(egui::Color32::RED),
        );
    }

    fn render_compound(&self, ui: &mut egui::Ui, _types: &Types) {
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
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types) {
        ui.label(
            egui::RichText::new(format!("Type '{}' not found", self.name))
                .color(egui::Color32::RED),
        );
    }

    fn render_compound(&self, _ui: &mut egui::Ui, _types: &Types) {}
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
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types) {
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

    fn render_compound(&self, _ui: &mut egui::Ui, _types: &Types) {}
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

struct EnumWidget<'a> {
    enum_decl: type_crawler::EnumDecl,
    instance: TypeInstance<'a>,
}

impl AsDataWidget for type_crawler::EnumDecl {
    fn as_data_widget<'a>(
        &self,
        _ui: &mut egui::Ui,
        _types: &Types,
        instance: TypeInstance<'a>,
    ) -> Box<dyn DataWidget + 'a> {
        Box::new(EnumWidget { enum_decl: self.clone(), instance })
    }
}

impl<'a> DataWidget for EnumWidget<'a> {
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types) {
        ui.label(egui::RichText::new("enum not implemented").color(egui::Color32::RED));
    }

    fn render_compound(&self, ui: &mut egui::Ui, _types: &Types) {
        ui.label(egui::RichText::new("enum compound not implemented").color(egui::Color32::RED));
    }
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
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types) {
        let mut open = self.is_open(ui);
        if ui.selectable_label(open, "Open").clicked() {
            open = !open;
            ui.ctx().data_mut(|data| data.insert_temp(self.open_id, open));
        }
    }

    fn render_compound(&self, ui: &mut egui::Ui, types: &Types) {
        ui.indent("struct_compound", |ui| {
            for field in self.struct_decl.fields() {
                let offset = field.offset_bytes();
                let size = field.kind().size(types);
                let field_instance = self.instance.slice(offset, size);

                ui.push_id(offset, |ui| {
                    let mut widget = field.kind().as_data_widget(ui, types, field_instance);
                    columns::columns(ui, &[1, 3, 2], |columns| {
                        render_value_badge(&mut columns[0], types, field.kind());
                        columns[1].label(field.name().unwrap_or(""));
                        widget.render_value(&mut columns[2], types);
                    });
                    if widget.is_open(ui) {
                        widget.render_compound(ui, types);
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
    fn render_value(&mut self, ui: &mut egui::Ui, _types: &Types) {
        let mut open = self.is_open(ui);
        if ui.selectable_label(open, "Open").clicked() {
            open = !open;
            ui.ctx().data_mut(|data| data.insert_temp(self.open_id, open));
        }
    }

    fn render_compound(&self, ui: &mut egui::Ui, types: &Types) {
        ui.indent("union_compound", |ui| {
            for (i, field) in self.union_decl.fields().iter().enumerate() {
                let size = field.kind().size(types);
                let field_instance = self.instance.slice(0, size);

                ui.push_id(i, |ui| {
                    let mut widget = field.kind().as_data_widget(ui, types, field_instance);
                    columns::columns(ui, &[1, 3, 2], |columns| {
                        render_value_badge(&mut columns[0], types, field.kind());
                        columns[1].label(field.name().unwrap_or(""));
                        widget.render_value(&mut columns[2], types);
                    });
                    if widget.is_open(ui) {
                        widget.render_compound(ui, types);
                    }
                });
            }
        });
    }

    fn is_open(&self, ui: &mut egui::Ui) -> bool {
        ui.ctx().data_mut(|data| data.get_temp::<bool>(self.open_id).unwrap_or(false))
    }
}

fn render_value_badge(
    ui: &mut egui::Ui,
    types: &Types,
    kind: &type_crawler::TypeKind,
) -> egui::Response {
    let (text, background, color) = value_badge_style(types, kind);
    ui.label(
        egui::RichText::new(text)
            .background_color(egui::Color32::from_hex(background).unwrap())
            .color(egui::Color32::from_hex(color).unwrap()),
    )
}

fn value_badge_style<'a>(
    types: &'a Types,
    kind: &'a type_crawler::TypeKind,
) -> (Cow<'a, str>, &'a str, &'a str) {
    match kind {
        type_crawler::TypeKind::USize { .. } => ("usize".into(), "#224eff", "#ffffff"),
        type_crawler::TypeKind::SSize { .. } => ("ssize".into(), "#ff4e22", "#ffffff"),
        type_crawler::TypeKind::U64 => ("u64".into(), "#0033ff", "#ffffff"),
        type_crawler::TypeKind::U32 => ("u32".into(), "#466bff", "#ffffff"),
        type_crawler::TypeKind::U16 => ("u16".into(), "#7691ff", "#ffffff"),
        type_crawler::TypeKind::U8 => ("u8".into(), "#a9baff", "#000000"),
        type_crawler::TypeKind::S64 => ("s64".into(), "#ff3300", "#ffffff"),
        type_crawler::TypeKind::S32 => ("s32".into(), "#ff6b46", "#000000"),
        type_crawler::TypeKind::S16 => ("s16".into(), "#ff9176", "#000000"),
        type_crawler::TypeKind::S8 => ("s8".into(), "#ffbaa9", "#000000"),
        type_crawler::TypeKind::Bool => ("bool".into(), "#008d00", "#ffffff"),
        type_crawler::TypeKind::Void => ("void".into(), "#242424", "#ffffff"),
        type_crawler::TypeKind::Pointer { pointee_type, .. } => {
            let (text, background, color) = value_badge_style(types, pointee_type);
            (format!("{text}*").into(), background, color)
        }
        type_crawler::TypeKind::Array { element_type, .. } => {
            let (text, background, color) = value_badge_style(types, element_type);
            (format!("{text}[]").into(), background, color)
        }
        type_crawler::TypeKind::Function { .. } => ("fn".into(), "#35620bff", "#ffffff"),
        type_crawler::TypeKind::Struct(struct_decl) => {
            (struct_decl.name().unwrap_or("struct").into(), "#af1cc9", "#ffffff")
        }
        type_crawler::TypeKind::Union(union_decl) => {
            (union_decl.name().unwrap_or("union").into(), "#c9bb1c", "#000000")
        }
        type_crawler::TypeKind::Named(name) => match name.as_str() {
            "q20" => ("q20".into(), "#006abb", "#ffffff"),
            _ => {
                let Some(type_decl) = types.get(name) else {
                    return ("unknown".into(), "#000000ff", "#ffffff");
                };
                match type_decl {
                    type_crawler::TypeDecl::Typedef(typedef) => {
                        value_badge_style(types, typedef.underlying_type())
                    }
                    type_crawler::TypeDecl::Enum(_) => ("enum".into(), "#ff8c00", "#ffffff"),
                    type_crawler::TypeDecl::Struct(_) => ("struct".into(), "#af1cc9", "#ffffff"),
                    type_crawler::TypeDecl::Union(_) => ("union".into(), "#c9bb1c", "#000000"),
                }
            }
        },
    }
}
