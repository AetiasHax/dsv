use eframe::egui::{self, Rect, pos2, vec2};

pub fn weighted_columns<R>(
    ui: &mut egui::Ui,
    column_weights: &[usize],
    add_contents: impl FnOnce(&mut [egui::Ui]) -> R,
) -> R {
    weighted_columns_dyn(ui, column_weights, Box::new(add_contents))
}

fn weighted_columns_dyn<'c, R>(
    ui: &mut egui::Ui,
    column_weights: &[usize],
    add_contents: Box<dyn FnOnce(&mut [egui::Ui]) -> R + 'c>,
) -> R {
    let spacing = ui.spacing().item_spacing.x;
    let num_columns = column_weights.len();
    let total_spacing = spacing * (num_columns as f32 - 1.0);
    let columns_width = ui.available_width() - total_spacing;
    let total_column_weight = column_weights.iter().sum::<usize>() as f32;
    let top_left = ui.cursor().min;

    let mut columns = Vec::with_capacity(num_columns);
    let mut x = 0.0;
    for &weight in column_weights {
        let column_width = weight as f32 / total_column_weight * columns_width;
        let pos = top_left + vec2(x, 0.0);
        let child_rect =
            Rect::from_min_max(pos, pos2(pos.x + column_width, ui.max_rect().right_bottom().y));
        let mut column_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(child_rect)
                .layout(egui::Layout::top_down_justified(egui::Align::LEFT)),
        );
        column_ui.set_width(column_width);
        columns.push(column_ui);
        x += column_width + spacing;
    }

    let result = add_contents(&mut columns[..]);

    let mut total_width = 0.0;
    let mut max_height = 0.0;
    for column in &columns {
        total_width += column.min_rect().width();
        max_height = column.min_size().y.max(max_height);
    }

    // Make sure we fit everything next frame:
    let total_required_width = total_spacing + total_width;

    let size = vec2(ui.available_width().max(total_required_width), max_height);
    ui.advance_cursor_after_rect(Rect::from_min_size(top_left, size));
    result
}

pub fn fixed_columns<R>(
    ui: &mut egui::Ui,
    column_widths: &[f32],
    add_contents: impl FnOnce(&mut [egui::Ui]) -> R,
) -> R {
    fixed_columns_dyn(ui, column_widths, Box::new(add_contents))
}

fn fixed_columns_dyn<'c, R>(
    ui: &mut egui::Ui,
    column_widths: &[f32],
    add_contents: Box<dyn FnOnce(&mut [egui::Ui]) -> R + 'c>,
) -> R {
    let spacing = ui.spacing().item_spacing.x;
    let num_columns = column_widths.len();
    let total_spacing = spacing * (num_columns as f32 - 1.0);
    let top_left = ui.cursor().min;

    let mut columns = Vec::with_capacity(num_columns);
    let mut x = 0.0;
    for &width in column_widths {
        let pos = top_left + vec2(x, 0.0);
        let child_rect =
            Rect::from_min_max(pos, pos2(pos.x + width, ui.max_rect().right_bottom().y));
        let mut column_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(child_rect)
                .layout(egui::Layout::top_down_justified(egui::Align::LEFT)),
        );
        column_ui.set_width(width);
        columns.push(column_ui);
        x += width + spacing;
    }

    let result = add_contents(&mut columns[..]);

    let mut total_width = 0.0;
    let mut max_height = 0.0;
    for column in &columns {
        total_width += column.min_rect().width();
        max_height = column.min_size().y.max(max_height);
    }

    // Make sure we fit everything next frame:
    let total_required_width = total_spacing + total_width;

    let size = vec2(ui.available_width().max(total_required_width), max_height);
    ui.advance_cursor_after_rect(Rect::from_min_size(top_left, size));
    result
}
