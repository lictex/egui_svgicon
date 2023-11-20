use lyon::lyon_tessellation::StrokeOptions;
use lyon::path::*;

pub fn to_lyon_stroke(stroke: &usvg::Stroke) -> StrokeOptions {
    let linecap = match stroke.linecap {
        usvg::LineCap::Butt => LineCap::Butt,
        usvg::LineCap::Square => LineCap::Square,
        usvg::LineCap::Round => LineCap::Round,
    };
    let linejoin = match stroke.linejoin {
        usvg::LineJoin::Miter => LineJoin::Miter,
        usvg::LineJoin::Bevel => LineJoin::Bevel,
        usvg::LineJoin::Round => LineJoin::Round,
        usvg::LineJoin::MiterClip => LineJoin::MiterClip,
    };
    StrokeOptions::default()
        .with_line_width(stroke.width.get() as f32)
        .with_line_cap(linecap)
        .with_line_join(linejoin)
}
pub fn to_egui_color(color: usvg::Color, opacity: f32) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        color.red,
        color.green,
        color.blue,
        (opacity * 255.0) as u8,
    )
}
pub fn to_egui_rect(rect: usvg::NonZeroRect) -> egui::Rect {
    egui::Rect::from_min_max(
        [rect.left() as f32, rect.top() as f32].into(),
        [rect.right() as f32, rect.bottom() as f32].into(),
    )
}
