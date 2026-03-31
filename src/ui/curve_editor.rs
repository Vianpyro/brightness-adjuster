use eframe::egui;

use crate::curve;
use crate::weather;

use super::SettingsApp;

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

impl SettingsApp {
    pub(crate) fn draw_curve_editor(
        &mut self,
        ui: &mut egui::Ui,
        _current_elevation: f64,
        day_progress: f64,
        weather_forecast: &[(f64, f64)],
        cloud_attenuation: f64,
    ) {
        let editable = self.active_curve_is_editable();
        let sense = if editable {
            egui::Sense::click_and_drag()
        } else {
            egui::Sense::hover()
        };

        let plot_w = ui.available_width().min(440.0);
        let margin_left = 36.0_f32;
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

        self.draw_grid(&painter, &plot_rect, grid_color, text_color, &font);
        self.draw_reference_curve(&painter, &plot_rect);

        let display_curve = self.active_curve_display();
        let blue = egui::Color32::from_rgb(55, 138, 221);

        self.draw_user_curve(
            &painter,
            &plot_rect,
            &display_curve,
            blue,
            weather_forecast,
            cloud_attenuation,
        );
        self.draw_now_indicator(
            &painter,
            &plot_rect,
            &display_curve,
            day_progress,
            weather_forecast,
            cloud_attenuation,
            blue,
        );
        self.draw_control_points(&painter, &plot_rect, &display_curve, blue);

        if editable {
            self.handle_interaction(ui, &response, &display_curve, &plot_rect);
        }
    }

    fn draw_grid(
        &self,
        painter: &egui::Painter,
        rect: &egui::Rect,
        grid_color: egui::Color32,
        text_color: egui::Color32,
        font: &egui::FontId,
    ) {
        for pct in [25, 50, 75] {
            let y = bright_to_y(pct as f64, rect);
            painter.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                egui::Stroke::new(0.3, grid_color),
            );
            painter.text(
                egui::pos2(rect.left() - 4.0, y),
                egui::Align2::RIGHT_CENTER,
                format!("{pct}%"),
                font.clone(),
                text_color,
            );
        }
        painter.text(
            egui::pos2(rect.left() - 4.0, rect.bottom()),
            egui::Align2::RIGHT_CENTER,
            "0%",
            font.clone(),
            text_color,
        );
        painter.text(
            egui::pos2(rect.left() - 4.0, rect.top()),
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
            let x = progress_to_x(frac, rect);
            let is_major = frac == 0.0 || frac == 0.5 || frac == 1.0;
            if frac > 0.0 && frac < 1.0 {
                let segs = 40;
                for s in 0..segs {
                    if s % 2 == 1 {
                        continue;
                    }
                    let y1 = rect.top() + s as f32 * rect.height() / segs as f32;
                    let y2 = rect.top() + (s + 1) as f32 * rect.height() / segs as f32;
                    painter.line_segment(
                        [egui::pos2(x, y1), egui::pos2(x, y2)],
                        egui::Stroke::new(if is_major { 0.5 } else { 0.3 }, grid_color),
                    );
                }
            }
            painter.line_segment(
                [
                    egui::pos2(x, rect.bottom()),
                    egui::pos2(x, rect.bottom() + 4.0),
                ],
                egui::Stroke::new(0.5, grid_color),
            );
            painter.text(
                egui::pos2(x, rect.bottom() + 6.0),
                egui::Align2::CENTER_TOP,
                label,
                font.clone(),
                text_color,
            );
        }
    }

    fn draw_reference_curve(&self, painter: &egui::Painter, rect: &egui::Rect) {
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
                        progress_to_x(p1, rect),
                        bright_to_y(curve::natural_reference(p1), rect),
                    ),
                    egui::pos2(
                        progress_to_x(p2, rect),
                        bright_to_y(curve::natural_reference(p2), rect),
                    ),
                ],
                egui::Stroke::new(1.0, amber),
            );
        }
    }

    fn draw_user_curve(
        &self,
        painter: &egui::Painter,
        rect: &egui::Rect,
        display_curve: &curve::BrightnessCurve,
        blue: egui::Color32,
        weather_forecast: &[(f64, f64)],
        cloud_attenuation: f64,
    ) {
        if !weather_forecast.is_empty() {
            let green = egui::Color32::from_rgba_premultiplied(80, 190, 80, 180);
            let points: Vec<egui::Pos2> = (0..=200)
                .map(|i| {
                    let p = i as f64 / 200.0;
                    let base = display_curve.evaluate(p);
                    let cloud = weather::interpolate_cloud_cover(weather_forecast, p);
                    let effective = (base * (1.0 - cloud * cloud_attenuation)).clamp(0.0, 100.0);
                    egui::pos2(progress_to_x(p, rect), bright_to_y(effective, rect))
                })
                .collect();

            if points.len() >= 2 {
                painter.add(egui::Shape::line(points, egui::Stroke::new(2.0, green)));
            }

            let base_pts: Vec<egui::Pos2> = (0..=200)
                .map(|i| {
                    let p = i as f64 / 200.0;
                    egui::pos2(
                        progress_to_x(p, rect),
                        bright_to_y(display_curve.evaluate(p), rect),
                    )
                })
                .collect();
            for i in (0..base_pts.len() - 1).step_by(4) {
                let end = (i + 2).min(base_pts.len() - 1);
                painter.line_segment(
                    [base_pts[i], base_pts[end]],
                    egui::Stroke::new(1.0, blue.gamma_multiply(0.4)),
                );
            }
        } else {
            let points: Vec<egui::Pos2> = (0..=200)
                .map(|i| {
                    let p = i as f64 / 200.0;
                    egui::pos2(
                        progress_to_x(p, rect),
                        bright_to_y(display_curve.evaluate(p), rect),
                    )
                })
                .collect();
            if points.len() >= 2 {
                painter.add(egui::Shape::line(points, egui::Stroke::new(2.0, blue)));
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_now_indicator(
        &self,
        painter: &egui::Painter,
        rect: &egui::Rect,
        display_curve: &curve::BrightnessCurve,
        day_progress: f64,
        weather_forecast: &[(f64, f64)],
        cloud_attenuation: f64,
        blue: egui::Color32,
    ) {
        if day_progress <= 0.001 || day_progress >= 0.999 {
            return;
        }

        let now_x = progress_to_x(day_progress, rect);
        let now_bright = if !weather_forecast.is_empty() {
            let base = display_curve.evaluate(day_progress);
            let cloud = weather::interpolate_cloud_cover(weather_forecast, day_progress);
            (base * (1.0 - cloud * cloud_attenuation)).clamp(0.0, 100.0)
        } else {
            display_curve.evaluate(day_progress)
        };
        let now_y = bright_to_y(now_bright, rect);

        let red = egui::Color32::from_rgba_premultiplied(220, 60, 60, 140);
        let segs = 20;
        for s in 0..segs {
            if s % 2 == 1 {
                continue;
            }
            let y1 = rect.top() + s as f32 * rect.height() / segs as f32;
            let y2 = rect.top() + (s + 1) as f32 * rect.height() / segs as f32;
            painter.line_segment(
                [egui::pos2(now_x, y1), egui::pos2(now_x, y2)],
                egui::Stroke::new(1.0, red),
            );
        }

        let dot_color = if weather_forecast.is_empty() {
            blue
        } else {
            egui::Color32::from_rgb(80, 190, 80)
        };
        painter.circle_filled(egui::pos2(now_x, now_y), 5.0, dot_color);
        painter.circle_stroke(
            egui::pos2(now_x, now_y),
            5.0,
            egui::Stroke::new(1.5, egui::Color32::from_gray(30)),
        );
    }

    fn draw_control_points(
        &self,
        painter: &egui::Painter,
        rect: &egui::Rect,
        display_curve: &curve::BrightnessCurve,
        blue: egui::Color32,
    ) {
        let point_radius = 6.0_f32;

        for (i, pt) in display_curve.points.iter().enumerate() {
            let pos = egui::pos2(
                progress_to_x(pt.position, rect),
                bright_to_y(pt.brightness, rect),
            );
            let is_dragged = self.dragging_point == Some(i);
            let r = if is_dragged {
                point_radius + 2.0
            } else {
                point_radius
            };
            painter.circle_filled(pos, r, egui::Color32::from_gray(30));
            painter.circle_stroke(
                pos,
                r,
                egui::Stroke::new(if is_dragged { 2.5 } else { 1.5 }, blue),
            );
        }
    }

    fn handle_interaction(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        display_curve: &curve::BrightnessCurve,
        plot_rect: &egui::Rect,
    ) {
        let snap_enabled = !ui.input(|i| i.modifiers.shift);
        let pointer_pos = response.interact_pointer_pos();
        let point_hit_radius = 12.0_f32;

        if response.drag_started()
            && let Some(pos) = pointer_pos
        {
            self.dragging_point = None;
            let mut min_dist = point_hit_radius;
            for (i, pt) in display_curve.points.iter().enumerate() {
                let screen_pt = egui::pos2(
                    progress_to_x(pt.position, plot_rect),
                    bright_to_y(pt.brightness, plot_rect),
                );
                if pos.distance(screen_pt) < min_dist {
                    min_dist = pos.distance(screen_pt);
                    self.dragging_point = Some(i);
                }
            }
        }

        if response.dragged()
            && let (Some(idx), Some(pos)) = (self.dragging_point, pointer_pos)
            && let Some(c) = self.active_curve_mut()
        {
            let n = c.points.len();
            let raw_pos = x_to_progress(pos.x, plot_rect);
            let raw_bright = y_to_bright(pos.y, plot_rect);

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
            let position = x_to_progress(pos.x, plot_rect);
            let brightness = y_to_bright(pos.y, plot_rect);
            if let Some(c) = self.active_curve_mut() {
                c.add_point(position, brightness);
            }
        }

        if response.secondary_clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let mut nearest = None;
            let mut min_dist = point_hit_radius;

            for (i, pt) in display_curve.points.iter().enumerate() {
                let screen_pt = egui::pos2(
                    progress_to_x(pt.position, plot_rect),
                    bright_to_y(pt.brightness, plot_rect),
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
