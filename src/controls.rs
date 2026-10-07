//! Shared paper controls: a single action hierarchy across Atlas.
use crate::washi::{INK, LINE, MUTED, PANEL, PAPER};
use eframe::egui::*;

pub fn action(ui: &mut Ui, text: impl Into<String>, primary: bool) -> Response {
    let text = RichText::new(text.into())
        .size(14.)
        .color(if primary { PAPER } else { INK });
    let response = ui.add(
        Button::new(text)
            .fill(if primary { INK } else { PANEL })
            .stroke(Stroke::NONE)
            .corner_radius(5)
            .min_size(vec2(0., 32.)),
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.expand(2.),
            6,
            Stroke::new(1.5, INK),
            StrokeKind::Outside,
        );
    }
    response
}
pub fn tab(ui: &mut Ui, text: &str, selected: bool) -> Response {
    let response = ui.add(
        Button::new(
            RichText::new(text)
                .size(14.)
                .color(if selected { INK } else { MUTED }),
        )
        .fill(if selected {
            PANEL
        } else {
            Color32::TRANSPARENT
        })
        .stroke(Stroke::NONE)
        .corner_radius(5)
        .min_size(vec2(0., 34.)),
    );
    if selected {
        ui.painter().line_segment(
            [
                response.rect.left_bottom() + vec2(10., -2.),
                response.rect.right_bottom() + vec2(-10., -2.),
            ],
            Stroke::new(1.5, INK),
        );
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.expand(1.),
            5,
            Stroke::new(1.5, INK),
            StrokeKind::Outside,
        );
    }
    response
}
pub fn field(ui: &mut Ui, label: &str, value: &mut String, multiline: bool) -> Response {
    let label = ui.label(RichText::new(label).size(13.).color(MUTED));
    let frame = Frame::new()
        .fill(PAPER)
        .stroke(Stroke::new(1., LINE))
        .corner_radius(5)
        .inner_margin(8);
    let edit = if multiline {
        TextEdit::multiline(value).desired_rows(3)
    } else {
        TextEdit::singleline(value)
    };
    let response = ui
        .add(
            edit.font(FontId::proportional(15.))
                .text_color(INK)
                .desired_width(f32::INFINITY)
                .frame(frame),
        )
        .labelled_by(label.id);
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.expand(1.),
            5,
            Stroke::new(1.5, INK),
            StrokeKind::Outside,
        );
    }
    response
}
