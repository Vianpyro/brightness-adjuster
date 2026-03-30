use anyhow::{Context, Result};
use brightness::blocking::{Brightness, brightness_devices};
use chrono::{Local, NaiveTime};
use serde::Deserialize;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use crate::config::SharedState;
use crate::curve;

#[derive(Debug, Deserialize)]
pub struct AstronomyResponse {
    pub sunrise: String,
    pub sunset: String,
    pub solar_noon: String,
}

fn fetch_astronomy(api_key: &str) -> Result<AstronomyResponse> {
    let url = format!("https://api.ipgeolocation.io/astronomy?apiKey={api_key}");
    reqwest::blocking::get(&url)
        .context("Failed to reach ipgeolocation.io")?
        .json()
        .context("Failed to parse astronomy JSON")
}

fn parse_time(s: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(s.trim(), "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(s.trim(), "%-H:%M"))
        .context(format!("Cannot parse time '{s}'"))
}

fn compute_sun_position(now: NaiveTime, sunrise: NaiveTime, sunset: NaiveTime) -> (f64, f64) {
    if now <= sunrise || now >= sunset {
        return (0.0, if now >= sunset { 1.0 } else { 0.0 });
    }
    let day_secs = (sunset - sunrise).num_seconds() as f64;
    if day_secs <= 0.0 {
        return (0.0, 0.0);
    }
    let progress = (now - sunrise).num_seconds() as f64 / day_secs;
    let elevation = (progress * std::f64::consts::PI).sin();
    (elevation, progress)
}

fn set_all_displays(target: u32) {
    for dev in brightness_devices().flatten() {
        let _ = dev.set(target);
    }
}

fn fade_brightness(from: u32, to: u32) {
    if from == to {
        return;
    }
    let step: i32 = if to > from { 1 } else { -1 };
    let mut value = from as i32;
    while value != to as i32 {
        value += step;
        set_all_displays(value as u32);
        thread::sleep(Duration::from_millis(15));
    }
}

pub fn run_loop(state: Arc<SharedState>) {
    let mut sun_times: Option<(NaiveTime, NaiveTime)> = None;

    loop {
        let config = state.config.read().unwrap().clone();

        if state.needs_refetch.swap(false, Ordering::Relaxed) || sun_times.is_none() {
            let monitors = curve::list_display_names();
            *state.detected_monitors.write().unwrap() = monitors;

            if config.api_key.is_empty() {
                let sr = NaiveTime::from_hms_opt(6, 0, 0).unwrap();
                let ss = NaiveTime::from_hms_opt(18, 0, 0).unwrap();
                sun_times = Some((sr, ss));
                *state.sunrise_str.write().unwrap() = "06:00".into();
                *state.noon_str.write().unwrap() = "12:00".into();
                *state.sunset_str.write().unwrap() = "18:00".into();
                state.set_status("Running (default times — add API key for accuracy)");
            } else {
                state.set_status("Fetching astronomy data...");

                match fetch_astronomy(&config.api_key) {
                    Ok(astro) => {
                        *state.sunrise_str.write().unwrap() = astro.sunrise.clone();
                        *state.noon_str.write().unwrap() = astro.solar_noon.clone();
                        *state.sunset_str.write().unwrap() = astro.sunset.clone();

                        match (parse_time(&astro.sunrise), parse_time(&astro.sunset)) {
                            (Ok(sr), Ok(ss)) => {
                                sun_times = Some((sr, ss));
                                state.set_status("Running");
                            }
                            _ => {
                                state.set_status("Error parsing sun times");
                                thread::sleep(Duration::from_secs(10));
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        state.set_status(&format!("API error: {e}"));
                        thread::sleep(Duration::from_secs(30));
                        continue;
                    }
                }
            }
        }

        if let Some((sunrise, sunset)) = sun_times {
            let now = Local::now().time();
            let (elevation, day_progress) = compute_sun_position(now, sunrise, sunset);

            *state.current_elevation.write().unwrap() = elevation;
            *state.current_day_progress.write().unwrap() = day_progress;

            let global_target = config.global_curve.evaluate(day_progress) as u32;

            if config.monitors.is_empty() {
                let current = state.current_brightness.load(Ordering::Relaxed);
                if current != global_target {
                    fade_brightness(current, global_target);
                    state
                        .current_brightness
                        .store(global_target, Ordering::Relaxed);
                }
            } else {
                for dev in brightness_devices().flatten() {
                    let name = dev.device_name().unwrap_or_default();
                    let override_entry = config.monitors.iter().find(|m| m.name == name);

                    let target = match override_entry {
                        Some(m) => m.evaluate(day_progress, &config.global_curve),
                        None => Some(global_target as f64),
                    };

                    if let Some(t) = target {
                        let _ = dev.set(t as u32);
                    }
                }
                state
                    .current_brightness
                    .store(global_target, Ordering::Relaxed);
            }
        }

        thread::sleep(Duration::from_secs(config.update_interval_secs));
    }
}
