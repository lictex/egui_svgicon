use eframe::*;
use egui::*;
use egui_svgicon::*;

const ICON: &[u8] = include_bytes!("test.svg");

fn main() {
    #[cfg(feature = "puffin")]
    puffin::set_scopes_on(true);

    eframe::run_native(
        "svg icons test",
        eframe::NativeOptions {
            initial_window_size: Some(egui::vec2(854.0, 480.0)),
            default_theme: Theme::Light,
            multisampling: 8,
            ..Default::default()
        },
        Box::new(|_cc| Box::new(Test(8))),
    )
    .unwrap()
}

struct Test(usize);
impl eframe::App for Test {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(feature = "puffin")]
        puffin::GlobalProfiler::lock().new_frame();

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("original");
                        Svg::new(ICON).show(ui);
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label("color");
                        Svg::new(ICON).with_color(Color32::LIGHT_BLUE).show(ui);
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label("random");
                        Svg::new(ICON)
                            .with_color({
                                let hasher = egui::epaint::ahash::RandomState::new();
                                Color32::from_rgb(
                                    (hasher.hash_one(0) % 256) as u8,
                                    (hasher.hash_one(1) % 256) as u8,
                                    (hasher.hash_one(2) % 256) as u8,
                                )
                            })
                            .show(ui);
                    });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    let size = [96.0, 144.0];
                    macro_rules! rect {
                        ($u:expr,$a:expr) => {
                            $u.painter().rect_stroke(
                                Rect::from_min_size($u.cursor().min, $a.into()),
                                Rounding::none(),
                                Stroke {
                                    width: 1.0,
                                    color: Color32::LIGHT_GRAY,
                                },
                            );
                        };
                    }
                    ui.vertical(|ui| {
                        ui.label("unset");
                        rect!(ui, size);
                        Svg::new(ICON)
                            .with_fit_mode(FitMode::None)
                            .show_sized(ui, size);
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label("factor");
                        rect!(ui, size);
                        Svg::new(ICON)
                            .with_fit_mode(FitMode::Factor(0.5))
                            .show_sized(ui, size);
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label("size");
                        rect!(ui, size);
                        Svg::new(ICON)
                            .with_fit_mode(FitMode::Size(Vec2::new(16.0, 16.0)))
                            .show_sized(ui, size);
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label("contain");
                        rect!(ui, size);
                        Svg::new(ICON)
                            .with_fit_mode(FitMode::Contain(Margin::default()))
                            .show_sized(ui, size);
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label("contain margin");
                        let margin = Margin {
                            left: 16.0,
                            right: 4.0,
                            top: 4.0,
                            bottom: 16.0,
                        };
                        rect!(ui, size);
                        ui.painter().rect_stroke(
                            {
                                let mut rect = Rect::from_min_size(ui.cursor().min, size.into());
                                rect.min += margin.left_top();
                                rect.max -= margin.right_bottom();
                                rect
                            },
                            Rounding::none(),
                            Stroke {
                                width: 1.0,
                                color: Color32::DARK_RED,
                            },
                        );
                        Svg::new(ICON)
                            .with_fit_mode(FitMode::Contain(margin))
                            .show_sized(ui, size);
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label("cover");
                        rect!(ui, size);
                        Svg::new(ICON)
                            .with_fit_mode(FitMode::Cover)
                            .show_sized(ui, size);
                    });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.set_height(64.0);
                    ui.label("inline");
                    Svg::new(ICON).show(ui);
                    ui.label("icons");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.set_height(64.0);
                    ui.label("inline");
                    Svg::new(ICON).show_justified(ui);
                    ui.label("icons");
                    Svg::new(ICON).show_justified(ui);
                    ui.label("justified");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.set_height(64.0);
                    ui.label("inline");
                    Svg::new(ICON).show_sized(ui, [32.0, 32.0]);
                    ui.label("icons");
                    Svg::new(ICON).show_sized(ui, [32.0, 32.0]);
                    ui.label("sized");
                });
                ui.separator();
                let response = Svg::new(ICON)
                    .with_color_from_style()
                    .with_sense(Sense::click_and_drag())
                    .show(ui);
                ui.horizontal(|ui| {
                    ui.label("sense: ");
                    if response.hovered() {
                        ui.label("hovered");
                    }
                    if response.dragged() {
                        ui.label("dragged");
                    }
                });
                ui.separator();
                ui.label("gradient");
                ui.horizontal(|ui| {
                    ui.set_height(64.0);
                    Svg::new(include_bytes!("test_gradient.svg")).show_justified(ui);
                    Svg::new(include_bytes!("test_gradient.svg"))
                        .with_gradient(
                            &[
                                (0.0, Color32::TRANSPARENT),
                                (0.25, Color32::BLACK),
                                (0.5, Color32::RED),
                                (1.0, Color32::RED),
                            ],
                            Pos2::ZERO,
                            Pos2::new(32.0, 32.0),
                            TextureWrapMode::Clamp,
                        )
                        .show_justified(ui);
                    Svg::new(include_bytes!("test_gradient_overflow.svg"))
                        .with_tolerance(0.01)
                        .show_justified(ui);
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("many");
                    ui.add(DragValue::new(&mut self.0));
                });
                ui.horizontal_wrapped(|ui| {
                    for _ in 0..self.0 {
                        Svg::new(ICON).show(ui);
                    }
                });
            });
        });

        #[cfg(feature = "puffin")]
        puffin_egui::profiler_window(ctx);
    }
}
