use eframe::egui;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tray_icon::menu::{MenuEvent, MenuId};

use crate::config::{self, SharedState};

pub struct SettingsApp {
    state: Arc<SharedState>,

    settings_menu_id: MenuId,
    quit_menu_id: MenuId,

    visible: bool,
    first_frame: bool,

    api_key_input: String,
    show_api_key: bool,
    min_brightness: u32,
    max_brightness: u32,
    update_interval_mins: u64,
    start_on_startup: bool,
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
            min_brightness: config.min_brightness,
            max_brightness: config.max_brightness,
            update_interval_mins: config.update_interval_secs / 60,
            start_on_startup: config.start_on_startup,
        }
    }

    fn load_fields_from_config(&mut self) {
        let cfg = self.state.config.read().unwrap().clone();
        self.api_key_input = cfg.api_key;
        self.min_brightness = cfg.min_brightness;
        self.max_brightness = cfg.max_brightness;
        self.update_interval_mins = cfg.update_interval_secs / 60;
        self.start_on_startup = cfg.start_on_startup;
    }

    fn save_and_apply(&mut self) {
        let new_cfg = config::Config {
            api_key: self.api_key_input.clone(),
            min_brightness: self.min_brightness,
            max_brightness: self.max_brightness,
            update_interval_secs: self.update_interval_mins * 60,
            start_on_startup: self.start_on_startup,
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
            ui.heading("Sunrise Brightness");
            ui.add_space(6.0);

            egui::Grid::new("settings")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("API key:");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.api_key_input)
                                .password(!self.show_api_key)
                                .desired_width(220.0),
                        );
                        if ui
                            .button(if self.show_api_key { "Hide" } else { "Show" })
                            .clicked()
                        {
                            self.show_api_key = !self.show_api_key;
                        }
                    });
                    ui.end_row();

                    ui.label("Min brightness:");
                    let mut min = self.min_brightness as i32;
                    ui.add(egui::Slider::new(&mut min, 0..=99).suffix(" %"));
                    self.min_brightness = min as u32;
                    ui.end_row();

                    ui.label("Max brightness:");
                    let mut max = self.max_brightness as i32;
                    ui.add(egui::Slider::new(&mut max, 1..=100).suffix(" %"));
                    self.max_brightness = max as u32;
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

            ui.add_space(4.0);
            if self.min_brightness >= self.max_brightness {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 80, 80),
                    "Min must be less than max",
                );
            }
            if self.api_key_input.trim().is_empty() {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 200, 60),
                    "An API key is required",
                );
            }

            ui.separator();

            let status = self.state.status.read().unwrap().clone();
            let brightness = self.state.current_brightness.load(Ordering::Relaxed);
            let sunrise = self.state.sunrise_str.read().unwrap().clone();
            let noon = self.state.noon_str.read().unwrap().clone();

            egui::Grid::new("status")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Status:");
                    ui.label(&status);
                    ui.end_row();

                    ui.label("Brightness:");
                    ui.label(format!("{brightness} %"));
                    ui.end_row();

                    if !sunrise.is_empty() {
                        ui.label("Sunrise:");
                        ui.label(&sunrise);
                        ui.end_row();

                        ui.label("Solar noon:");
                        ui.label(&noon);
                        ui.end_row();
                    }
                });

            ui.separator();

            ui.horizontal(|ui| {
                let valid = self.min_brightness < self.max_brightness
                    && !self.api_key_input.trim().is_empty();

                if ui
                    .add_enabled(valid, egui::Button::new("Save & Apply"))
                    .clicked()
                {
                    self.save_and_apply();
                }

                if ui.button("Close").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    self.visible = false;
                }
            });
        });
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
