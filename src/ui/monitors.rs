use eframe::egui;

use crate::curve::{self, BrightnessCurve, MonitorOverride};

use super::SettingsApp;

impl SettingsApp {
    pub(crate) fn draw_monitor_tabs(&mut self, ui: &mut egui::Ui, monitors: &[String]) {
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

    pub(crate) fn draw_monitor_mode_selector(&mut self, ui: &mut egui::Ui) {
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

    pub(crate) fn draw_preset_buttons(&mut self, ui: &mut egui::Ui) {
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
}
