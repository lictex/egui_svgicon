use egui::*;
use utils::*;

#[cfg(feature = "gradient")]
mod gradient;
mod tessellation;
mod utils;

/// ???
#[cfg(feature = "cached")]
macro_rules! bytes {
    ($t:expr, $T:ty) => {
        unsafe { std::mem::transmute::<$T, [u8; std::mem::size_of::<$T>()]>($t) }
    };
}

#[derive(Clone, Copy)]
pub enum FitMode {
    None,
    Size(Vec2),
    Factor(f32),
    Cover,
    Contain(Margin),
}

#[derive(Clone, Copy)]
pub enum TextureWrapMode {
    Clamp,
    Repeat,
    Mirror,
}

enum ColorOverride {
    None,
    FromStyle,
    Color(Color32),
    Texture(TextureId),
    #[cfg(feature = "gradient")]
    Gradient(gradient::Gradient),
}

enum Background {
    None,
    FromStyle,
    Custom {
        fill: Color32,
        rounding: Rounding,
        stroke: Stroke,
    },
}

#[cfg(not(feature = "cached"))]
type SvgTree = usvg::Tree;
#[cfg(feature = "cached")]
type SvgTree = (u64, std::rc::Rc<usvg::Tree>);

pub struct Svg {
    tree: SvgTree,
    color_override: ColorOverride,
    background: Background,
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
            background: _,
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
        use usvg::TreeParsing;

        #[cfg(feature = "puffin")]
        puffin::profile_function!();

        #[cfg(not(feature = "cached"))]
        let tree = usvg::Tree::from_data(data, &usvg::Options::default()).unwrap();

        #[cfg(feature = "cached")]
        let tree = {
            use egui::epaint::ahash::*;
            use std::cell::RefCell;
            use std::hash::BuildHasher;
            use std::hash::Hash;
            use std::hash::Hasher;
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
            color_override: ColorOverride::None,
            background: Background::None,
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
        self.color_override = ColorOverride::Color(color);
        self
    }
    /// override all elements' color with given texture
    pub fn with_texture(mut self, texture: TextureId) -> Self {
        self.color_override = ColorOverride::Texture(texture);
        self
    }
    /// override all elements' color with given gradient
    pub fn with_gradient(
        self,
        colors: &[(f32, Color32)],
        start: Pos2,
        end: Pos2,
        wrap_mode: TextureWrapMode,
    ) -> Self {
        #[cfg(not(feature = "gradient"))]
        {
            let _ = (colors, start, end, wrap_mode);
            self
        }
        #[cfg(feature = "gradient")]
        {
            let mut svg = self;
            svg.color_override = ColorOverride::Gradient(gradient::Gradient {
                colors: colors
                    .iter()
                    .copied()
                    .map(|(fac, color)| gradient::GradientColor { fac, color })
                    .collect(),
                start,
                end,
                wrap_mode,
            });
            svg
        }
    }
    /// override all elements' color with fg_stroke
    pub fn with_color_from_style(mut self) -> Self {
        self.color_override = ColorOverride::FromStyle;
        self
    }
    /// set background
    pub fn with_background(mut self, rounding: Rounding, fill: Color32, stroke: Stroke) -> Self {
        self.background = Background::Custom {
            fill,
            rounding,
            stroke,
        };
        self
    }
    /// set background from style
    pub fn with_background_from_style(mut self) -> Self {
        self.background = Background::FromStyle;
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
        let mut size = self.svg_rect().size();
        if let FitMode::Contain(m) = self.fit_mode {
            size += m.sum();
        }
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
        let mut shape = tessellation::tessellate(&self, rect, size / self.svg_rect().size());

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
                    tessellation::tessellate(
                        svg,
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
        macro_rules! svg_pos {
            ($v:expr) => {
                (($v.pos - rect.min) * (self.svg_rect().size() / rect.size())
                    + self.svg_rect().min.to_vec2())
                .to_pos2()
            };
        }
        match &self.color_override {
            ColorOverride::None => {}
            ColorOverride::FromStyle => {
                shape
                    .vertices
                    .iter_mut()
                    .for_each(|v| v.color = ui.style().interact(&response).fg_stroke.color);
            }
            ColorOverride::Color(c) => shape.vertices.iter_mut().for_each(|v| v.color = *c),
            ColorOverride::Texture(t) => {
                shape.texture_id = *t;
                shape.vertices.iter_mut().for_each(|v| {
                    v.color = Color32::WHITE;
                    v.uv = (svg_pos!(v).to_vec2() / self.svg_rect().size()).to_pos2();
                });
            }
            #[cfg(feature = "gradient")]
            ColorOverride::Gradient(g) => {
                shape
                    .vertices
                    .iter_mut()
                    .for_each(|v| v.color = g.color_at_pos(svg_pos!(v)));
            }
        };

        match &self.background {
            Background::None => {}
            Background::FromStyle => {
                let visual = ui.style().interact(&response);
                ui.painter().rect(
                    frame_rect,
                    visual.rounding,
                    visual.bg_fill,
                    visual.bg_stroke,
                );
            }
            Background::Custom {
                fill,
                rounding,
                stroke,
            } => ui.painter().rect(frame_rect, *rounding, *fill, *stroke),
        }

        ui.painter().with_clip_rect(frame_rect).add(shape);

        response
    }
    /// original viewbox of the svg shape
    pub fn svg_rect(&self) -> Rect {
        #[cfg(not(feature = "cached"))]
        let tree = &self.tree;
        #[cfg(feature = "cached")]
        let tree = &self.tree.1;

        to_egui_rect(tree.view_box.rect)
    }
}
