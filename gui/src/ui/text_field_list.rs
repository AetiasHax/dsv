use eframe::egui::{self, Widget};

pub struct TextFieldList<'a> {
    id: &'static str,
    list: &'a mut Vec<String>,
    field_hint: Option<&'static str>,
    add_button_text: Option<&'static str>,
}

#[derive(Default)]
pub struct TextFieldListResponse {
    pub changed: bool,
}

impl<'a> TextFieldList<'a> {
    pub fn new(id: &'static str, list: &'a mut Vec<String>) -> Self {
        Self { id, list, field_hint: None, add_button_text: None }
    }

    pub fn with_field_hint(mut self, hint: &'static str) -> Self {
        self.field_hint = Some(hint);
        self
    }

    pub fn with_add_button_text(mut self, text: &'static str) -> Self {
        self.add_button_text = Some(text);
        self
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> TextFieldListResponse {
        let mut response = TextFieldListResponse::default();
        let mut remove_index = None;
        egui_extras::TableBuilder::new(ui)
            .id_salt(self.id)
            .striped(true)
            .column(egui_extras::Column::exact(220.0))
            .column(egui_extras::Column::exact(50.0))
            .body(|mut body| {
                for i in 0..self.list.len() {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            let mut text_edit =
                                egui::TextEdit::singleline(&mut self.list[i]).desired_width(200.0);
                            if let Some(hint) = self.field_hint {
                                text_edit = text_edit.hint_text(hint);
                            }
                            if text_edit.show(ui).response.lost_focus() {
                                response.changed = true;
                            }
                        });
                        row.col(|ui| {
                            if egui::Button::new("Remove")
                                .wrap_mode(egui::TextWrapMode::Extend)
                                .ui(ui)
                                .clicked()
                            {
                                remove_index = Some(i);
                            }
                        });
                    });
                }
                body.row(20.0, |mut row| {
                    row.col(|ui| {
                        if ui.button(self.add_button_text.unwrap_or("Add")).clicked() {
                            self.list.push(String::new());
                        }
                    });
                });
            });
        if let Some(index) = remove_index {
            self.list.remove(index);
            response.changed = true;
        }
        response
    }
}
