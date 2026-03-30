use anyhow::{Context, Result};
use brightness::{blocking::Brightness, blocking::brightness_devices};
use chrono::{Local, NaiveTime, TimeDelta};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use crate::config::{Config, SharedState};

#[derive(Debug, Deserialize)]
pub struct AstronomyResponse {
    pub sunrise: String,
    pub solar_noon: String,
}

fn fetch_astronomy(api_key: &str) -> Result<AstronomyResponse> {
    let url = format!("https://api.ipgeolocation.io/astronomy?apiKey={api_key}");
    reqwest::blocking::get(&url)
        .context("Failed to reach ipgeolocation.io")?
        .json()
        .context("Failed to parse astronomy JSON")
}

pub type BrightnessSpans = BTreeMap<NaiveTime, u32>;

fn parse_time(s: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(s.trim(), "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(s.trim(), "%-H:%M"))
        .context(format!("Cannot parse time '{s}'"))
}

fn compute_spans(config: &Config, sunrise: &str, solar_noon: &str) -> Result<BrightnessSpans> {
    let sunrise_t = parse_time(sunrise)?;
    let noon_t = parse_time(solar_noon)?;

    let range = config.max_brightness - config.min_brightness;
    if range == 0 {
        anyhow::bail!("min and max brightness must differ");
    }

    let total_minutes = (noon_t - sunrise_t).num_seconds() as f64 / 60.0;
    let step_duration = total_minutes / range as f64;
    let num_steps = range * 2 + 1;

    let mut spans = BTreeMap::new();
    for i in 0..num_steps {
        let delta_ms = (step_duration * i as f64 * 60_000.0) as i64;
        let time = sunrise_t + TimeDelta::milliseconds(delta_ms);

        let cycle = (i % (range * 2)) as i64 - range as i64;
        let brightness = config.min_brightness as f64
            + range as f64 * (1.0 - cycle.unsigned_abs() as f64 / range as f64);

        spans.insert(time, brightness as u32);
    }
    Ok(spans)
}

fn brightness_at(spans: &BrightnessSpans, now: NaiveTime) -> u32 {
    spans
        .range(..=now)
        .next_back()
        .or_else(|| spans.iter().next())
        .map(|(_, &b)| b)
        .unwrap_or(100)
}

fn set_all_displays(target: u32) {
    for result in brightness_devices() {
        if let Ok(mut dev) = result {
            let _ = dev.set(target);
        }
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
    let mut spans: Option<BrightnessSpans> = None;

    loop {
        let config = state.config.read().unwrap().clone();

        if config.api_key.is_empty() {
            state.set_status("Waiting for API key...");
            thread::sleep(Duration::from_secs(3));
            continue;
        }

        if state.needs_refetch.swap(false, Ordering::Relaxed) || spans.is_none() {
            state.set_status("Fetching astronomy data...");

            match fetch_astronomy(&config.api_key) {
                Ok(astro) => {
                    *state.sunrise_str.write().unwrap() = astro.sunrise.clone();
                    *state.noon_str.write().unwrap() = astro.solar_noon.clone();

                    match compute_spans(&config, &astro.sunrise, &astro.solar_noon) {
                        Ok(s) => {
                            spans = Some(s);
                            state.set_status("Running");
                        }
                        Err(e) => {
                            state.set_status(&format!("Compute error: {e}"));
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

        if let Some(ref s) = spans {
            let now = Local::now().time();
            let target = brightness_at(s, now);
            let current = state.current_brightness.load(Ordering::Relaxed);

            if current != target {
                fade_brightness(current, target);
                state.current_brightness.store(target, Ordering::Relaxed);
            }
        }

        thread::sleep(Duration::from_secs(config.update_interval_secs));
    }
}
