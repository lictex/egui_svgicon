use crate::*;
use lyon::geom::euclid::Vector2D;
use lyon::geom::Line;
use lyon::math::Point;

pub struct GradientColor {
    pub fac: f32,
    pub color: Color32,
}

pub struct Gradient {
    pub colors: Vec<GradientColor>,
    pub start: Pos2,
    pub end: Pos2,
    pub wrap_mode: TextureWrapMode,
}
impl Gradient {
    pub fn new(g: &usvg::LinearGradient, transform: usvg::Transform) -> Self {
        let gradient_transform = append_transform(transform, g.transform);
        let ((x1, y1), (x2, y2)) = (
            gradient_transform.apply(g.x1, g.y1),
            gradient_transform.apply(g.x2, g.y2),
        );
        Gradient {
            colors: g
                .stops
                .iter()
                .map(|f| GradientColor {
                    fac: f.offset.get() as _,
                    color: to_egui_color(f.color, f.opacity.get()),
                })
                .collect(),
            start: Pos2::new(x1 as _, y1 as _),
            end: Pos2::new(x2 as _, y2 as _),
            wrap_mode: match g.spread_method {
                usvg::SpreadMethod::Pad => TextureWrapMode::Clamp,
                usvg::SpreadMethod::Reflect => TextureWrapMode::Mirror,
                usvg::SpreadMethod::Repeat => TextureWrapMode::Repeat,
            },
        }
    }
    pub fn color_at_pos(&self, pos: Pos2) -> Color32 {
        let fac = {
            let line = Line {
                point: Point::new(self.start.x, self.start.y),
                vector: Vector2D::new(-(self.end.x - self.start.x), self.end.y - self.start.y).yx(),
            };
            line.signed_distance_to_point(&Point::new(pos.x, pos.y)) / line.vector.length()
        };

        let fac = match self.wrap_mode {
            TextureWrapMode::Clamp => fac,
            TextureWrapMode::Mirror => 1.0 - (fac.abs() % 2.0 - 1.0).abs(),
            TextureWrapMode::Repeat => fac - fac.floor(),
        };

        let mut local_fac = 1.0;
        let [mut color_a, mut color_b] =
            [self.colors.last().map(|f| f.color).unwrap_or_default(); 2];
        for color in self.colors.windows(2) {
            color_a = color[0].color;
            color_b = color[1].color;
            if fac < color[1].fac {
                local_fac = (fac - color[0].fac) / (color[1].fac - color[0].fac);
                break;
            }
        }
        macro_rules! mix {
            ($a:expr,$b:expr,$f:expr) => {{
                let mut _r = $a;
                _r = (($a as f64) * (1.0 as f64 - $f as f64) + ($b as f64) * ($f as f64)) as _;
                _r
            }};
        }
        Color32::from_rgba_premultiplied(
            mix!(color_a.r(), color_b.r(), local_fac),
            mix!(color_a.g(), color_b.g(), local_fac),
            mix!(color_a.b(), color_b.b(), local_fac),
            mix!(color_a.a(), color_b.a(), local_fac),
        )
    }
}
