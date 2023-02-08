use crate::*;
use lyon::lyon_tessellation::geometry_builder::*;
use lyon::lyon_tessellation::*;
use lyon::math::Point;
use lyon::path::PathEvent;

pub fn tessellate(svg: &Svg, rect: Rect, scale: Vec2) -> Mesh {
    #[cfg(feature = "puffin")]
    puffin::profile_function!();

    #[cfg(not(feature = "cached"))]
    let tree = &svg.tree;
    #[cfg(feature = "cached")]
    let tree = &svg.tree.1;

    let mut buffer = VertexBuffers::<_, u32>::new();
    tessellate_recursive(
        svg,
        scale,
        rect,
        &mut buffer,
        &mut FillTessellator::new(),
        &mut StrokeTessellator::new(),
        &tree.root,
        Default::default(),
    );

    let mut mesh = Mesh::default();
    std::mem::swap(&mut buffer.vertices, &mut mesh.vertices);
    std::mem::swap(&mut buffer.indices, &mut mesh.indices);
    mesh
}
fn tessellate_recursive(
    svg: &Svg,
    scale: Vec2,
    rect: Rect,
    buffer: &mut VertexBuffers<epaint::Vertex, u32>,
    fill_tesselator: &mut FillTessellator,
    stroke_tesselator: &mut StrokeTessellator,
    parent: &usvg::Node,
    parent_transform: usvg::Transform,
) {
    for node in parent.children() {
        match &*node.borrow() {
            usvg::NodeKind::Path(p) => {
                let new_egui_vertex =
                    |point: Point, paint: &usvg::Paint, opacity: f64| -> epaint::Vertex {
                        let transform = append_transform(parent_transform, p.transform);
                        let svg_pos = {
                            let (x, y) = transform.apply(point.x as _, point.y as _);
                            Pos2::new(x as _, y as _)
                        };
                        let egui_pos = {
                            let mut pos = svg_pos;
                            pos -= svg.svg_rect().min.to_vec2();
                            pos.x *= scale.x;
                            pos.y *= scale.y;
                            pos += rect.min.to_vec2();
                            pos
                        };
                        epaint::Vertex {
                            pos: egui_pos,
                            uv: Pos2::ZERO,
                            color: {
                                match paint {
                                    usvg::Paint::Color(c) => to_egui_color(*c, opacity),
                                    #[cfg(feature = "gradient")]
                                    usvg::Paint::LinearGradient(g) => {
                                        gradient::Gradient::new(g, transform).color_at_pos(svg_pos)
                                    }
                                    _ => Color32::BLACK,
                                }
                            },
                        }
                    };
                let tolerance = if svg.scale_tolerance {
                    svg.tolerance / scale.max_elem()
                } else {
                    svg.tolerance
                };
                if let Some(fill) = &p.fill {
                    fill_tesselator
                        .tessellate(
                            PathConvIter::new(p),
                            &FillOptions::tolerance(tolerance),
                            &mut BuffersBuilder::new(buffer, |f: FillVertex| {
                                new_egui_vertex(f.position(), &fill.paint, fill.opacity.get())
                            }),
                        )
                        .unwrap();
                }
                if let Some(stroke) = &p.stroke {
                    stroke_tesselator
                        .tessellate(
                            PathConvIter::new(p),
                            &to_lyon_stroke(stroke).with_tolerance(tolerance),
                            &mut BuffersBuilder::new(buffer, |f: StrokeVertex| {
                                new_egui_vertex(f.position(), &stroke.paint, stroke.opacity.get())
                            }),
                        )
                        .unwrap();
                }
            }
            usvg::NodeKind::Group(g) => tessellate_recursive(
                svg,
                scale,
                rect,
                buffer,
                fill_tesselator,
                stroke_tesselator,
                &node,
                append_transform(parent_transform, g.transform),
            ),
            usvg::NodeKind::Image(_) | usvg::NodeKind::Text(_) => {}
        }
    }
}

// https://github.com/nical/lyon/blob/f097646635a4df9d99a51f0d81b538e3c3aa1adf/examples/wgpu_svg/src/main.rs#L677
pub struct PathConvIter<'a> {
    iter: usvg::PathSegmentsIter<'a>,
    prev: Point,
    first: Point,
    needs_end: bool,
    deferred: Option<PathEvent>,
}
impl<'l> Iterator for PathConvIter<'l> {
    type Item = PathEvent;
    fn next(&mut self) -> Option<PathEvent> {
        if self.deferred.is_some() {
            return self.deferred.take();
        }

        let next = self.iter.next();
        match next {
            Some(usvg::PathSegment::MoveTo { x, y }) => {
                if self.needs_end {
                    let last = self.prev;
                    let first = self.first;
                    self.needs_end = false;
                    self.prev = Point::new(x as f32, y as f32);
                    self.deferred = Some(PathEvent::Begin { at: self.prev });
                    self.first = self.prev;
                    Some(PathEvent::End {
                        last,
                        first,
                        close: false,
                    })
                } else {
                    self.first = Point::new(x as f32, y as f32);
                    self.needs_end = true;
                    Some(PathEvent::Begin { at: self.first })
                }
            }
            Some(usvg::PathSegment::LineTo { x, y }) => {
                self.needs_end = true;
                let from = self.prev;
                self.prev = Point::new(x as f32, y as f32);
                Some(PathEvent::Line {
                    from,
                    to: self.prev,
                })
            }
            Some(usvg::PathSegment::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            }) => {
                self.needs_end = true;
                let from = self.prev;
                self.prev = Point::new(x as f32, y as f32);
                Some(PathEvent::Cubic {
                    from,
                    ctrl1: Point::new(x1 as f32, y1 as f32),
                    ctrl2: Point::new(x2 as f32, y2 as f32),
                    to: self.prev,
                })
            }
            Some(usvg::PathSegment::ClosePath) => {
                self.needs_end = false;
                self.prev = self.first;
                Some(PathEvent::End {
                    last: self.prev,
                    first: self.first,
                    close: true,
                })
            }
            None => {
                if self.needs_end {
                    self.needs_end = false;
                    let last = self.prev;
                    let first = self.first;
                    Some(PathEvent::End {
                        last,
                        first,
                        close: false,
                    })
                } else {
                    None
                }
            }
        }
    }
}
impl<'l> PathConvIter<'l> {
    pub fn new(path: &'l usvg::Path) -> Self {
        PathConvIter {
            iter: path.data.segments(),
            first: Point::new(0.0, 0.0),
            prev: Point::new(0.0, 0.0),
            deferred: None,
            needs_end: false,
        }
    }
}
