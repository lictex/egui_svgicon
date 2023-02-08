use egui::*;
use lyon::lyon_tessellation::geometry_builder::*;
use lyon::lyon_tessellation::*;
use lyon::math::Point;
use lyon::path::PathEvent;

/// ???
#[cfg(feature = "cached")]
macro_rules! bytes {
    ($t:expr, $T:ty) => {
        unsafe { std::mem::transmute::<$T, [u8; std::mem::size_of::<$T>()]>($t) }
    };
}
macro_rules! append_transform {
    ($a:expr,$b:expr) => {{
        let mut transform = $a;
        transform.append(&$b);
        transform
    }};
}

#[derive(Clone, Copy)]
pub enum FitMode {
    None,
    Size(Vec2),
    Factor(f32),
    Cover,
    Contain(Margin),
}

#[cfg(not(feature = "cached"))]
type SvgTree = usvg::Tree;
#[cfg(feature = "cached")]
type SvgTree = (u64, std::rc::Rc<usvg::Tree>);

pub struct Svg {
    tree: SvgTree,
    color_override: Option<Color32>,
    color_from_style: bool,
    tolerance: f32,
    scale_tolerance: bool,
    fit_mode: FitMode,
    sense: Sense,
}
#[cfg(feature = "cached")]
impl std::hash::Hash for Svg {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let Self {
            tree: (key, _),
            color_override: _,
            color_from_style: _,
            tolerance,
            scale_tolerance,
            fit_mode,
            sense: _,
        } = self;
        key.hash(state);
        bytes!(*tolerance, f32).hash(state);
        scale_tolerance.hash(state);
        match fit_mode {
            FitMode::None => 0usize.hash(state),
            FitMode::Size(s) => {
                1usize.hash(state);
                bytes!(*s, Vec2).hash(state);
            }
            FitMode::Factor(f) => {
                2usize.hash(state);
                bytes!(*f, f32).hash(state);
            }
            FitMode::Cover => 3usize.hash(state),
            FitMode::Contain(margin) => {
                4usize.hash(state);
                bytes!(*margin, Margin).hash(state);
            }
        }
    }
}
impl Svg {
    /// load a svg icon from buffer
    #[cfg_attr(feature = "cached", doc = "")]
    #[cfg_attr(feature = "cached", doc = "`cached`: cached svg tree will never drop")]
    #[cfg_attr(feature = "static_cached", doc = "")]
    #[cfg_attr(
        feature = "static_cached",
        doc = "`static_cached`: using ptr as cache key so `data` must be `'static`"
    )]
    pub fn new(
        #[cfg(not(feature = "static_cached"))] data: &[u8],
        #[cfg(feature = "static_cached")] data: &'static [u8],
    ) -> Self {
        #[cfg(feature = "puffin")]
        puffin::profile_function!();

        #[cfg(not(feature = "cached"))]
        let tree = usvg::Tree::from_data(data, &usvg::Options::default()).unwrap();

        #[cfg(feature = "cached")]
        let tree = {
            use egui::epaint::ahash::*;
            use std::cell::RefCell;
            use std::hash::*;
            use std::rc::Rc;

            thread_local! {
                static CACHE: RefCell<HashMap<u64, Rc<usvg::Tree>>> = Default::default();
            }
            CACHE.with(|cache| {
                let key = {
                    let mut hasher = RandomState::with_seed(0).build_hasher();

                    #[cfg(not(feature = "static_cached"))]
                    data.hash(&mut hasher);

                    #[cfg(feature = "static_cached")]
                    data.as_ptr().hash(&mut hasher);

                    hasher.finish()
                };

                (
                    key,
                    cache
                        .borrow_mut()
                        .entry(key)
                        .or_insert_with(|| {
                            Rc::new(usvg::Tree::from_data(data, &usvg::Options::default()).unwrap())
                        })
                        .clone(),
                )
            })
        };

        Svg {
            tree,
            color_override: None,
            color_from_style: false,
            tolerance: 1.0,
            scale_tolerance: true,
            fit_mode: FitMode::Contain(Default::default()),
            sense: Sense::hover(),
        }
    }
    /// set the tessellation tolerance
    pub fn with_tolerance(mut self, tolerance: f32) -> Self {
        self.tolerance = tolerance;
        self
    }
    /// set whether the tessellation tolerance is affected by the scale
    pub fn with_scale_tolerance(mut self, scale_tolerance: bool) -> Self {
        self.scale_tolerance = scale_tolerance;
        self
    }
    /// override all elements' color
    pub fn with_color(mut self, color: Color32) -> Self {
        self.color_override = Some(color);
        self
    }
    /// override all elements' color with fg_stroke
    pub fn with_color_from_style(mut self, from_style: bool) -> Self {
        self.color_from_style = from_style;
        self
    }
    /// set how the shape fits into the frame
    pub fn with_fit_mode(mut self, fit_mode: FitMode) -> Self {
        self.fit_mode = fit_mode;
        self
    }
    /// set response sense
    pub fn with_sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }
    /// show the icon at the svg's original size
    pub fn show(self, ui: &mut Ui) -> Response {
        let size = self.svg_rect().size();
        self.show_sized(ui, size)
    }
    /// show the icon. size is based on available height of the ui
    pub fn show_justified(self, ui: &mut Ui) -> Response {
        let size = [
            ui.available_height() * self.svg_rect().aspect_ratio(),
            ui.available_height(),
        ];
        self.show_sized(ui, size)
    }
    /// show the icon at the given size
    pub fn show_sized(self, ui: &mut Ui, size: impl Into<Vec2>) -> Response {
        #[cfg(feature = "puffin")]
        puffin::profile_function!();

        let size = size.into();
        let (id, frame_rect) = ui.allocate_space(size);
        let mut inner_frame_rect = frame_rect;
        let size = match self.fit_mode {
            FitMode::None => self.svg_rect().size(),
            FitMode::Size(s) => s,
            FitMode::Factor(f) => self.svg_rect().size() * f,
            FitMode::Cover => Vec2::from(
                if frame_rect.aspect_ratio() > self.svg_rect().aspect_ratio() {
                    [
                        frame_rect.width(),
                        self.svg_rect().height() * frame_rect.width() / self.svg_rect().width(),
                    ]
                } else {
                    [
                        self.svg_rect().width() * frame_rect.height() / self.svg_rect().height(),
                        frame_rect.height(),
                    ]
                },
            ),
            FitMode::Contain(margin) => {
                inner_frame_rect.min += margin.left_top();
                inner_frame_rect.max -= margin.right_bottom();
                Vec2::from(
                    if inner_frame_rect.aspect_ratio() > self.svg_rect().aspect_ratio() {
                        [
                            self.svg_rect().width() * inner_frame_rect.height()
                                / self.svg_rect().height(),
                            inner_frame_rect.height(),
                        ]
                    } else {
                        [
                            inner_frame_rect.width(),
                            self.svg_rect().height() * inner_frame_rect.width()
                                / self.svg_rect().width(),
                        ]
                    },
                )
            }
        };
        let rect = Align2::CENTER_CENTER.align_size_within_rect(size, inner_frame_rect);
        let response = ui.interact(frame_rect, id, self.sense);

        #[cfg(feature = "culled")]
        if !ui.clip_rect().intersects(rect) {
            return response;
        }

        #[cfg(not(feature = "cached"))]
        let mut shape = self.tessellate(rect, size / self.svg_rect().size());

        #[cfg(feature = "cached")]
        let mut shape = {
            use egui::util::cache::*;
            use std::hash::*;

            #[derive(Clone, Copy)]
            struct TessellateCacheKey<'l>(&'l Svg, Vec2);
            impl Hash for TessellateCacheKey<'_> {
                fn hash<H: Hasher>(&self, state: &mut H) {
                    let TessellateCacheKey(svg, size) = self;
                    svg.hash(state);
                    bytes!(*size, Vec2).hash(state);
                }
            }

            #[derive(Default)]
            struct Tessellator;
            impl ComputerMut<TessellateCacheKey<'_>, Mesh> for Tessellator {
                fn compute(&mut self, TessellateCacheKey(svg, size): TessellateCacheKey) -> Mesh {
                    svg.tessellate(
                        Rect::from_min_size(Pos2::ZERO, size),
                        size / svg.svg_rect().size(),
                    )
                }
            }

            let mut mesh = ui.memory_mut(|mem| {
                mem.caches
                    .cache::<FrameCache<_, Tessellator>>()
                    .get(TessellateCacheKey(&self, size))
            });
            mesh.translate(rect.min.to_vec2());
            mesh
        };

        if let Some(color) = self.color_override.or_else(|| {
            self.color_from_style
                .then(|| ui.style().interact(&response).fg_stroke.color)
        }) {
            shape.vertices.iter_mut().for_each(|f| f.color = color);
        }

        ui.painter().with_clip_rect(frame_rect).add(shape);

        response
    }

    fn svg_rect(&self) -> Rect {
        #[cfg(not(feature = "cached"))]
        let tree = &self.tree;
        #[cfg(feature = "cached")]
        let tree = &self.tree.1;

        tree.view_box.rect.convert()
    }
    fn tessellate(&self, rect: Rect, scale: Vec2) -> Mesh {
        #[cfg(feature = "puffin")]
        puffin::profile_function!();

        #[cfg(not(feature = "cached"))]
        let tree = &self.tree;
        #[cfg(feature = "cached")]
        let tree = &self.tree.1;

        let mut buffer = VertexBuffers::<_, u32>::new();
        self.tessellate_recursive(
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
        &self,
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
                            let transform = append_transform!(parent_transform, p.transform);
                            epaint::Vertex {
                                pos: {
                                    let mut pos = {
                                        let (x, y) = transform.apply(point.x as _, point.y as _);
                                        Vec2::new(x as _, y as _)
                                    };
                                    pos -= self.svg_rect().min.to_vec2();
                                    pos.x *= scale.x;
                                    pos.y *= scale.y;
                                    pos += rect.min.to_vec2();
                                    pos.to_pos2()
                                },
                                uv: Pos2::ZERO,
                                color: {
                                    let (color, opacity) = match paint {
                                        usvg::Paint::Color(c) => (*c, opacity),
                                        #[cfg(feature = "gradient")]
                                        usvg::Paint::LinearGradient(g) => {
                                            linear_gradient(g, point, transform)
                                        }
                                        _ => (usvg::Color::black(), 1.0),
                                    };
                                    (color, opacity).convert()
                                },
                            }
                        };
                    let tolerance = if self.scale_tolerance {
                        self.tolerance / scale.max_elem()
                    } else {
                        self.tolerance
                    };
                    if let Some(fill) = &p.fill {
                        fill_tesselator
                            .tessellate(
                                p.convert(),
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
                                p.convert(),
                                &stroke.convert().with_tolerance(tolerance),
                                &mut BuffersBuilder::new(buffer, |f: StrokeVertex| {
                                    new_egui_vertex(
                                        f.position(),
                                        &stroke.paint,
                                        stroke.opacity.get(),
                                    )
                                }),
                            )
                            .unwrap();
                    }
                }
                usvg::NodeKind::Group(g) => self.tessellate_recursive(
                    scale,
                    rect,
                    buffer,
                    fill_tesselator,
                    stroke_tesselator,
                    &node,
                    append_transform!(parent_transform, g.transform),
                ),
                usvg::NodeKind::Image(_) | usvg::NodeKind::Text(_) => {}
            }
        }
    }
}

#[cfg(feature = "gradient")]
fn linear_gradient(
    g: &usvg::LinearGradient,
    point: Point,
    transform: usvg::Transform,
) -> (usvg::Color, f64) {
    use lyon::geom::euclid::Vector2D;
    use lyon::geom::Line;

    let point = {
        let (x, y) = transform.apply(point.x as _, point.y as _);
        Point::new(x as _, y as _)
    };
    let fac = {
        let gradient_transform = append_transform!(transform, g.transform);
        let ((x1, y1), (x2, y2)) = (
            gradient_transform.apply(g.x1, g.y1),
            gradient_transform.apply(g.x2, g.y2),
        );
        let line = Line {
            point: Point::new(x1 as _, y1 as _),
            vector: Vector2D::new(-(x2 - x1) as _, (y2 - y1) as _).yx(),
        };
        line.signed_distance_to_point(&point) / line.vector.length()
    } as f64;

    let fac = match g.spread_method {
        usvg::SpreadMethod::Pad => fac,
        usvg::SpreadMethod::Reflect => 1.0 - (fac.abs() % 2.0 - 1.0).abs(),
        usvg::SpreadMethod::Repeat => fac - fac.floor(),
    };

    let mut local_fac = 0.0;
    let [(mut color_a, mut opacity_a), (mut color_b, mut opacity_b)] =
        [g.stops
            .first()
            .map(|f| (f.color, f.opacity.get()))
            .unwrap_or((usvg::Color::black(), 0.0)); 2];
    for stop in g.stops.windows(2) {
        let offset_a = stop[0].offset.get();
        let offset_b = stop[1].offset.get();
        if fac > offset_a {
            local_fac = (fac - offset_a) / (offset_b - offset_a);
            color_a = stop[0].color;
            opacity_a = stop[0].opacity.get();
            color_b = stop[1].color;
            opacity_b = stop[1].opacity.get();
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
    (
        usvg::Color::new_rgb(
            mix!(color_a.red, color_b.red, local_fac),
            mix!(color_a.green, color_b.green, local_fac),
            mix!(color_a.blue, color_b.blue, local_fac),
        ),
        mix!(opacity_a, opacity_b, local_fac),
    )
}

// https://github.com/nical/lyon/blob/f097646635a4df9d99a51f0d81b538e3c3aa1adf/examples/wgpu_svg/src/main.rs#L677
struct PathConvIter<'a> {
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

trait Convert<'l, T> {
    fn convert(&'l self) -> T;
}
impl Convert<'_, StrokeOptions> for usvg::Stroke {
    fn convert(&self) -> StrokeOptions {
        let linecap = match self.linecap {
            usvg::LineCap::Butt => LineCap::Butt,
            usvg::LineCap::Square => LineCap::Square,
            usvg::LineCap::Round => LineCap::Round,
        };
        let linejoin = match self.linejoin {
            usvg::LineJoin::Miter => LineJoin::Miter,
            usvg::LineJoin::Bevel => LineJoin::Bevel,
            usvg::LineJoin::Round => LineJoin::Round,
        };
        StrokeOptions::default()
            .with_line_width(self.width.get() as f32)
            .with_line_cap(linecap)
            .with_line_join(linejoin)
    }
}
impl<'l> Convert<'l, PathConvIter<'l>> for usvg::Path {
    fn convert(&'l self) -> PathConvIter<'l> {
        PathConvIter {
            iter: self.data.segments(),
            first: Point::new(0.0, 0.0),
            prev: Point::new(0.0, 0.0),
            deferred: None,
            needs_end: false,
        }
    }
}
impl Convert<'_, Color32> for (usvg::Color, f64) {
    fn convert(&self) -> Color32 {
        let (color, opacity) = *self;
        Color32::from_rgba_unmultiplied(color.red, color.green, color.blue, (opacity * 255.0) as u8)
    }
}
impl Convert<'_, Rect> for usvg::Rect {
    fn convert(&self) -> Rect {
        Rect::from_min_max(
            [self.left() as f32, self.top() as f32].into(),
            [self.right() as f32, self.bottom() as f32].into(),
        )
    }
}
