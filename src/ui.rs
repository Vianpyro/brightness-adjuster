use eframe::egui;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tray_icon::menu::{MenuEvent, MenuId};

use crate::config::{self, SharedState};
use crate::curve::{self, BrightnessCurve, MonitorOverride};

fn progress_to_x(progress: f64, rect: &egui::Rect) -> f32 {
    rect.left() + progress as f32 * rect.width()
}
fn bright_to_y(bright: f64, rect: &egui::Rect) -> f32 {
    rect.bottom() - (bright / 100.0) as f32 * rect.height()
}
fn x_to_progress(x: f32, rect: &egui::Rect) -> f64 {
    ((x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64
}
fn y_to_bright(y: f32, rect: &egui::Rect) -> f64 {
    (((rect.bottom() - y) / rect.height()) as f64 * 100.0).clamp(0.0, 100.0)
}

pub struct SettingsApp {
    state: Arc<SharedState>,
    settings_menu_id: MenuId,
    quit_menu_id: MenuId,
    visible: bool,
    first_frame: bool,

    // Editable fields
    api_key_input: String,
    show_api_key: bool,
    update_interval_mins: u64,
    start_on_startup: bool,
    global_curve: BrightnessCurve,
    monitor_overrides: Vec<MonitorOverride>,

    // UI state
    active_tab: usize,
    dragging_point: Option<usize>,
}

impl SettingsApp {
    pub fn new(state: Arc<SharedState>, settings_menu_id: MenuId, quit_menu_id: MenuId) -> Self {
        let config = state.config.read().unwrap().clone();
        Self {
            state,
            settings_menu_id,
            quit_menu_id,
            visible: false,
            first_frame: true,
            api_key_input: config.api_key,
            show_api_key: false,
            update_interval_mins: config.update_interval_secs / 60,
            start_on_startup: config.start_on_startup,
            global_curve: config.global_curve,
            monitor_overrides: config.monitors,
            active_tab: 0,
            dragging_point: None,
        }
    }

    fn load_fields_from_config(&mut self) {
        let cfg = self.state.config.read().unwrap().clone();
        self.api_key_input = cfg.api_key;
        self.update_interval_mins = cfg.update_interval_secs / 60;
        self.start_on_startup = cfg.start_on_startup;
        self.global_curve = cfg.global_curve;
        self.monitor_overrides = cfg.monitors;
        self.active_tab = 0;
        self.dragging_point = None;
    }

    fn save_and_apply(&mut self) {
        let new_cfg = config::Config {
            api_key: self.api_key_input.clone(),
            update_interval_secs: self.update_interval_mins * 60,
            start_on_startup: self.start_on_startup,
            global_curve: self.global_curve.clone(),
            monitors: self.monitor_overrides.clone(),
        };

        if let Err(e) = config::save_config(&new_cfg) {
            eprintln!("Failed to save config: {e}");
        }

        *self.state.config.write().unwrap() = new_cfg;
        self.state.needs_refetch.store(true, Ordering::Relaxed);

        if let Err(e) = toggle_autostart(self.start_on_startup) {
            eprintln!("Auto-start toggle failed: {e}");
        }
    }

    fn active_curve_mut(&mut self) -> Option<&mut BrightnessCurve> {
        if self.active_tab == 0 {
            Some(&mut self.global_curve)
        } else {
            let idx = self.active_tab - 1;
            self.monitor_overrides.get_mut(idx).and_then(|m| {
                if m.mode == "custom" {
                    m.curve.as_mut()
                } else {
                    None
                }
            })
        }
    }

    fn active_curve_display(&self) -> BrightnessCurve {
        if self.active_tab == 0 {
            return self.global_curve.clone();
        }
        let idx = self.active_tab - 1;
        match self.monitor_overrides.get(idx) {
            Some(m) if m.mode == "custom" => m.curve.clone().unwrap_or_default(),
            _ => self.global_curve.clone(),
        }
    }

    fn active_curve_is_editable(&self) -> bool {
        self.active_tab == 0
            || self
                .monitor_overrides
                .get(self.active_tab.wrapping_sub(1))
                .is_some_and(|m| m.mode == "custom")
    }
}

impl eframe::App for SettingsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if self.first_frame {
            self.first_frame = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.settings_menu_id {
                self.load_fields_from_config();
                self.visible = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            } else if event.id == self.quit_menu_id {
                std::process::exit(0);
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.visible = false;
        }

        ctx.request_repaint_after(Duration::from_millis(250));
        if !self.visible {
            return;
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(format!("Sunrise Brightness v{}", config::VERSION));

                    let latest = self.state.latest_version.read().unwrap().clone();
                    if let Some(ref latest) = latest && latest.as_str() != config::VERSION
                            && ui
                                .small_button(format!("v{latest} available"))
                                .on_hover_text("Open releases page")
                                .clicked()
                            {
                                ui.ctx().open_url(egui::OpenUrl {
                                    url: format!("{}/releases/latest", config::REPO_URL),
                                    new_tab: true,
                                });
                            }
                });
                ui.add_space(4.0);

                egui::Grid::new("settings_grid")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("API key:");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.api_key_input)
                                    .password(!self.show_api_key)
                                    .desired_width(200.0),
                            );
                            if ui.button(if self.show_api_key { "Hide" } else { "Show" }).clicked() {
                                self.show_api_key = !self.show_api_key;
                            }
                        });
                        ui.end_row();

                        ui.label("Update interval:");
                        let mut mins = self.update_interval_mins as i32;
                        ui.add(egui::Slider::new(&mut mins, 1..=30).suffix(" min"));
                        self.update_interval_mins = mins as u64;
                        ui.end_row();

                        ui.label("Start on boot:");
                        ui.checkbox(&mut self.start_on_startup, "");
                        ui.end_row();
                    });

                if self.api_key_input.trim().is_empty() {
                    ui.horizontal(|ui| {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 200, 60),
                            "API key needed for sun times.",
                        );
                        if ui.small_button("Get one free").clicked() {
                            ui.ctx().open_url(egui::OpenUrl {
                                url: "https://app.ipgeolocation.io/dashboard".into(),
                                new_tab: true,
                            });
                        }
                    });
                    ui.colored_label(
                        egui::Color32::GRAY,
                        "Without a key, default times are used (6:00-18:00).",
                    );
                }

                ui.separator();

                let monitors = self.state.detected_monitors.read().unwrap().clone();
                self.draw_monitor_tabs(ui, &monitors);

                ui.add_space(4.0);

                if self.active_tab > 0 {
                    self.draw_monitor_mode_selector(ui);
                    ui.add_space(4.0);
                }

                if self.active_curve_is_editable() {
                    self.draw_preset_buttons(ui);
                    ui.add_space(4.0);
                }

                let elevation = *self.state.current_elevation.read().unwrap();
                let day_progress = *self.state.current_day_progress.read().unwrap();

                let is_ignored = self.active_tab > 0
                    && self
                        .monitor_overrides
                        .get(self.active_tab.wrapping_sub(1))
                        .is_some_and(|m| m.mode == "ignored");

                if is_ignored {
                    ui.add_space(8.0);
                    ui.colored_label(
                        egui::Color32::GRAY,
                        "This display is ignored, brightness will not be adjusted.",
                    );
                    ui.add_space(8.0);
                } else {
                    self.draw_curve_editor(ui, elevation, day_progress);

                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(
                            "Drag points to reshape. Double-click to add. Right-click to remove. Hold Shift to disable grid snapping.",
                        )
                        .small()
                        .color(egui::Color32::GRAY),
                    );
                }

                ui.separator();

                let status = self.state.status.read().unwrap().clone();
                let brightness = self.state.current_brightness.load(Ordering::Relaxed);
                let sunrise = self.state.sunrise_str.read().unwrap().clone();
                let noon = self.state.noon_str.read().unwrap().clone();
                let sunset = self.state.sunset_str.read().unwrap().clone();

                egui::Grid::new("status_grid")
                    .num_columns(2)
                    .spacing([12.0, 3.0])
                    .show(ui, |ui| {
                        ui.label("Status:");
                        ui.label(&status);
                        ui.end_row();

                        ui.label("Brightness:");
                        ui.label(format!("{brightness} %"));
                        ui.end_row();

                        if !sunrise.is_empty() {
                            ui.label("Sun:");
                            ui.label(format!("{sunrise} / {noon} / {sunset}"));
                            ui.end_row();

                            ui.label("Elevation:");
                            ui.label(format!("{:.0} %", elevation * 100.0));
                            ui.end_row();
                        }
                    });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Save & Apply").clicked() {
                        self.save_and_apply();
                    }
                    if ui.button("Close").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                        self.visible = false;
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.label(
                        egui::RichText::new("Made by Vianpyro")
                            .small()
                            .color(egui::Color32::GRAY),
                    );
                    ui.label(
                        egui::RichText::new("·")
                            .small()
                            .color(egui::Color32::GRAY),
                    );
                    if ui
                        .small_button("GitHub")
                        .on_hover_text(config::REPO_URL)
                        .clicked()
                    {
                        ui.ctx().open_url(egui::OpenUrl {
                            url: config::REPO_URL.to_string(),
                            new_tab: true,
                        });
                    }
                });
            });
        });
    }
}

impl SettingsApp {
    fn draw_monitor_tabs(&mut self, ui: &mut egui::Ui, monitors: &[String]) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.active_tab == 0, "All displays")
                .clicked()
            {
                self.active_tab = 0;
                self.dragging_point = None;
            }

            for (i, raw_name) in monitors.iter().enumerate() {
                if !self.monitor_overrides.iter().any(|m| m.name == *raw_name) {
                    self.monitor_overrides
                        .push(MonitorOverride::new_global(raw_name.clone()));
                }

                let label = self
                    .monitor_overrides
                    .iter()
                    .find(|m| m.name == *raw_name)
                    .map(|m| m.display_label())
                    .unwrap_or_else(|| curve::clean_display_name(raw_name));

                let tab = ui.selectable_label(self.active_tab == i + 1, &label);

                tab.clone().on_hover_text(raw_name);

                if tab.clicked() {
                    self.active_tab = i + 1;
                    self.dragging_point = None;
                }
            }
        });
    }

    fn draw_monitor_mode_selector(&mut self, ui: &mut egui::Ui) {
        let idx = self.active_tab - 1;
        if idx >= self.monitor_overrides.len() {
            return;
        }

        ui.horizontal(|ui| {
            ui.label("Name:");
            let mut alias = self.monitor_overrides[idx]
                .alias
                .clone()
                .unwrap_or_default();
            let hint = curve::clean_display_name(&self.monitor_overrides[idx].name);
            let response = ui.add(
                egui::TextEdit::singleline(&mut alias)
                    .hint_text(&hint)
                    .desired_width(180.0),
            );
            if response.changed() {
                self.monitor_overrides[idx].alias =
                    if alias.is_empty() { None } else { Some(alias) };
            }
        });

        ui.horizontal(|ui| {
            ui.label("Mode:");

            let mode = self.monitor_overrides[idx].mode.clone();

            if ui.selectable_label(mode == "global", "Global").clicked() {
                self.monitor_overrides[idx].mode = "global".into();
            }
            if ui.selectable_label(mode == "offset", "Offset").clicked() {
                self.monitor_overrides[idx].mode = "offset".into();
            }
            if ui.selectable_label(mode == "custom", "Custom").clicked() {
                self.monitor_overrides[idx].mode = "custom".into();
                if self.monitor_overrides[idx].curve.is_none() {
                    self.monitor_overrides[idx].curve = Some(self.global_curve.clone());
                }
            }
            if ui
                .selectable_label(mode == "ignored", "Ignored")
                .on_hover_text("Don't touch this display")
                .clicked()
            {
                self.monitor_overrides[idx].mode = "ignored".into();
            }

            if mode == "offset" {
                let mut off = self.monitor_overrides[idx].offset;
                ui.add(egui::Slider::new(&mut off, -50..=50).suffix(" %"));
                self.monitor_overrides[idx].offset = off;
            }
        });
    }

    fn draw_preset_buttons(&mut self, ui: &mut egui::Ui) {
        let current_preset = self.active_curve_display().preset.clone();

        ui.horizontal(|ui| {
            ui.label("Presets:");
            for (id, label) in [
                ("natural", "Natural"),
                ("linear", "Linear"),
                ("night_owl", "Night owl"),
                ("relax", "Relax"),
                ("cinema", "Cinema"),
                ("paper", "Paper"),
                ("early_bird", "Early bird"),
            ] {
                let active = current_preset.as_deref() == Some(id);
                if ui.selectable_label(active, label).clicked() {
                    let new_curve = BrightnessCurve::from_preset(id);
                    if let Some(c) = self.active_curve_mut() {
                        *c = new_curve;
                    }
                }
            }
        });
    }

    fn draw_curve_editor(&mut self, ui: &mut egui::Ui, _current_elevation: f64, day_progress: f64) {
        let editable = self.active_curve_is_editable();
        let sense = if editable {
            egui::Sense::click_and_drag()
        } else {
            egui::Sense::hover()
        };

        let plot_w = ui.available_width().min(440.0);
        let margin_left = 32.0_f32;
        let margin_bottom = 22.0_f32;
        let plot_h = 180.0_f32;
        let total = egui::vec2(plot_w, plot_h + margin_bottom + 8.0);

        let (response, painter) = ui.allocate_painter(total, sense);

        let plot_rect = egui::Rect::from_min_size(
            response.rect.min + egui::vec2(margin_left, 4.0),
            egui::vec2(plot_w - margin_left - 8.0, plot_h),
        );

        painter.rect_filled(plot_rect, 4.0, ui.visuals().extreme_bg_color);
        painter.rect_stroke(
            plot_rect,
            4.0,
            egui::Stroke::new(0.5, ui.visuals().widgets.noninteractive.bg_stroke.color),
            egui::StrokeKind::Outside,
        );

        let grid_color = ui
            .visuals()
            .widgets
            .noninteractive
            .bg_stroke
            .color
            .gamma_multiply(0.3);
        let text_color = ui.visuals().text_color().gamma_multiply(0.5);
        let font = egui::FontId::proportional(10.0);

        for pct in [25, 50, 75] {
            let y = bright_to_y(pct as f64, &plot_rect);
            painter.line_segment(
                [
                    egui::pos2(plot_rect.left(), y),
                    egui::pos2(plot_rect.right(), y),
                ],
                egui::Stroke::new(0.3, grid_color),
            );
            painter.text(
                egui::pos2(plot_rect.left() - 4.0, y),
                egui::Align2::RIGHT_CENTER,
                format!("{pct}%"),
                font.clone(),
                text_color,
            );
        }
        painter.text(
            egui::pos2(plot_rect.left() - 4.0, plot_rect.bottom()),
            egui::Align2::RIGHT_CENTER,
            "0%",
            font.clone(),
            text_color,
        );
        painter.text(
            egui::pos2(plot_rect.left() - 4.0, plot_rect.top()),
            egui::Align2::RIGHT_CENTER,
            "100%",
            font.clone(),
            text_color,
        );

        for (frac, label) in [
            (0.0, "sunrise"),
            (0.25, "25%"),
            (0.5, "zenith"),
            (0.75, "75%"),
            (1.0, "sunset"),
        ] {
            let x = progress_to_x(frac, &plot_rect);
            let is_major = frac == 0.0 || frac == 0.5 || frac == 1.0;
            if frac > 0.0 && frac < 1.0 {
                let segs = 40;
                for s in 0..segs {
                    if s % 2 == 1 {
                        continue;
                    }
                    let y1 = plot_rect.top() + s as f32 * plot_rect.height() / segs as f32;
                    let y2 = plot_rect.top() + (s + 1) as f32 * plot_rect.height() / segs as f32;
                    let w = if is_major { 0.5 } else { 0.3 };
                    painter.line_segment(
                        [egui::pos2(x, y1), egui::pos2(x, y2)],
                        egui::Stroke::new(w, grid_color),
                    );
                }
            }
            painter.line_segment(
                [
                    egui::pos2(x, plot_rect.bottom()),
                    egui::pos2(x, plot_rect.bottom() + 4.0),
                ],
                egui::Stroke::new(0.5, grid_color),
            );
            painter.text(
                egui::pos2(x, plot_rect.bottom() + 6.0),
                egui::Align2::CENTER_TOP,
                label,
                font.clone(),
                text_color,
            );
        }

        let amber = egui::Color32::from_rgba_premultiplied(200, 150, 40, 100);
        let steps = 120;
        for i in 0..steps {
            if i % 3 == 2 {
                continue;
            }
            let p1 = i as f64 / steps as f64;
            let p2 = (i + 1) as f64 / steps as f64;
            painter.line_segment(
                [
                    egui::pos2(
                        progress_to_x(p1, &plot_rect),
                        bright_to_y(curve::natural_reference(p1), &plot_rect),
                    ),
                    egui::pos2(
                        progress_to_x(p2, &plot_rect),
                        bright_to_y(curve::natural_reference(p2), &plot_rect),
                    ),
                ],
                egui::Stroke::new(1.0, amber),
            );
        }

        let display_curve = self.active_curve_display();
        let blue = egui::Color32::from_rgb(55, 138, 221);

        let curve_points: Vec<egui::Pos2> = (0..=200)
            .map(|i| {
                let p = i as f64 / 200.0;
                egui::pos2(
                    progress_to_x(p, &plot_rect),
                    bright_to_y(display_curve.evaluate(p), &plot_rect),
                )
            })
            .collect();

        if curve_points.len() >= 2 {
            painter.add(egui::Shape::line(
                curve_points,
                egui::Stroke::new(2.0, blue),
            ));
        }

        if day_progress > 0.001 && day_progress < 0.999 {
            let now_x = progress_to_x(day_progress, &plot_rect);
            let now_bright = display_curve.evaluate(day_progress);
            let now_y = bright_to_y(now_bright, &plot_rect);

            let red = egui::Color32::from_rgba_premultiplied(220, 60, 60, 140);
            let segs = 20;
            for s in 0..segs {
                if s % 2 == 1 {
                    continue;
                }
                let y1 = plot_rect.top() + s as f32 * plot_rect.height() / segs as f32;
                let y2 = plot_rect.top() + (s + 1) as f32 * plot_rect.height() / segs as f32;
                painter.line_segment(
                    [egui::pos2(now_x, y1), egui::pos2(now_x, y2)],
                    egui::Stroke::new(1.0, red),
                );
            }

            painter.circle_filled(egui::pos2(now_x, now_y), 5.0, blue);
            painter.circle_stroke(
                egui::pos2(now_x, now_y),
                5.0,
                egui::Stroke::new(1.5, ui.visuals().extreme_bg_color),
            );
        }

        let point_radius = 6.0_f32;
        let point_hit_radius = 12.0_f32;

        for (i, pt) in display_curve.points.iter().enumerate() {
            let pos = egui::pos2(
                progress_to_x(pt.position, &plot_rect),
                bright_to_y(pt.brightness, &plot_rect),
            );
            let is_dragged = self.dragging_point == Some(i);
            let r = if is_dragged {
                point_radius + 2.0
            } else {
                point_radius
            };

            painter.circle_filled(pos, r, ui.visuals().extreme_bg_color);
            painter.circle_stroke(
                pos,
                r,
                egui::Stroke::new(if is_dragged { 2.5 } else { 1.5 }, blue),
            );
        }

        if !editable {
            return;
        }

        let snap_enabled = !ui.input(|i| i.modifiers.shift);
        let pointer_pos = response.interact_pointer_pos();

        if response.drag_started()
            && let Some(pos) = pointer_pos
        {
            self.dragging_point = None;
            let mut min_dist = point_hit_radius;
            for (i, pt) in display_curve.points.iter().enumerate() {
                let screen_pt = egui::pos2(
                    progress_to_x(pt.position, &plot_rect),
                    bright_to_y(pt.brightness, &plot_rect),
                );
                let dist = pos.distance(screen_pt);
                if dist < min_dist {
                    min_dist = dist;
                    self.dragging_point = Some(i);
                }
            }
        }

        if response.dragged()
            && let (Some(idx), Some(pos)) = (self.dragging_point, pointer_pos)
            && let Some(c) = self.active_curve_mut()
        {
            let n = c.points.len();
            let raw_pos = x_to_progress(pos.x, &plot_rect);
            let raw_bright = y_to_bright(pos.y, &plot_rect);

            let position = if idx == 0 {
                0.0
            } else if idx == n - 1 {
                1.0
            } else if snap_enabled {
                curve::snap_to_grid(raw_pos, curve::SNAP_X, 1.0).clamp(0.01, 0.99)
            } else {
                raw_pos.clamp(0.01, 0.99)
            };

            let brightness = if snap_enabled {
                curve::snap_to_grid(raw_bright, curve::SNAP_Y, 100.0)
            } else {
                raw_bright
            };

            c.points[idx].position = position;
            c.points[idx].brightness = brightness;
            c.preset = None;
        }

        if response.drag_stopped() && self.dragging_point.is_some() {
            if let Some(c) = self.active_curve_mut() {
                c.sort();
            }
            self.dragging_point = None;
        }

        if response.double_clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let position = x_to_progress(pos.x, &plot_rect);
            let brightness = y_to_bright(pos.y, &plot_rect);
            if let Some(c) = self.active_curve_mut() {
                c.add_point(position, brightness);
            }
        }

        if response.secondary_clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let mut nearest = None;
            let mut min_dist = point_hit_radius;

            let pts = &self.active_curve_display().points;
            for (i, pt) in pts.iter().enumerate() {
                let screen_pt = egui::pos2(
                    progress_to_x(pt.position, &plot_rect),
                    bright_to_y(pt.brightness, &plot_rect),
                );
                let dist = pos.distance(screen_pt);
                if dist < min_dist {
                    min_dist = dist;
                    nearest = Some(i);
                }
            }
            if let Some(idx) = nearest
                && let Some(c) = self.active_curve_mut()
            {
                c.remove_point(idx);
            }
        }
    }
}

fn toggle_autostart(enable: bool) -> anyhow::Result<()> {
    let exe = std::env::current_exe()?.to_string_lossy().to_string();

    let launcher = auto_launch::AutoLaunchBuilder::new()
        .set_app_name("sunrise-brightness")
        .set_app_path(&exe)
        .build()
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if enable {
        launcher.enable().map_err(|e| anyhow::anyhow!("{e}"))?;
    } else {
        launcher.disable().map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    Ok(())
}
