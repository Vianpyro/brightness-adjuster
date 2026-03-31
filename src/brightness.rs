use brightness::blocking::{Brightness, brightness_devices};
use chrono::{DateTime, FixedOffset, Local, NaiveTime, TimeZone};
use serde::Deserialize;
use solar_positioning::{Horizon, spa, time::DeltaT, types::SunriseResult};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use crate::config::SharedState;
use crate::curve;

#[derive(Debug, Deserialize)]
struct IpApiResponse {
    lat: f64,
    lon: f64,
    #[serde(default)]
    city: String,
    #[serde(rename = "countryCode", default)]
    country_code: String,
}

fn detect_location() -> Option<(f64, f64, String)> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;
    let resp: IpApiResponse = client
        .get("http://ip-api.com/json/?fields=lat,lon,city,countryCode")
        .send()
        .ok()?
        .json()
        .ok()?;
    let label = if resp.city.is_empty() {
        format!("{:.2}, {:.2}", resp.lat, resp.lon)
    } else {
        format!("{}, {}", resp.city, resp.country_code)
    };
    Some((resp.lat, resp.lon, label))
}

struct SunTimes {
    sunrise: NaiveTime,
    transit: NaiveTime,
    sunset: NaiveTime,
}

fn compute_sun_times(lat: f64, lon: f64) -> Option<SunTimes> {
    let now: DateTime<FixedOffset> = Local::now().fixed_offset();
    let today_start = now.date_naive().and_hms_opt(0, 0, 0)?;
    let today = now.timezone().from_local_datetime(&today_start).single()?;
    let delta_t = DeltaT::estimate_from_date_like(today).unwrap_or(69.0);

    let result =
        spa::sunrise_sunset_for_horizon(today, lat, lon, delta_t, Horizon::SunriseSunset).ok()?;

    match result {
        SunriseResult::RegularDay {
            sunrise,
            transit,
            sunset,
        } => Some(SunTimes {
            sunrise: sunrise.with_timezone(&Local).time(),
            transit: transit.with_timezone(&Local).time(),
            sunset: sunset.with_timezone(&Local).time(),
        }),
        SunriseResult::AllDay { transit } => Some(SunTimes {
            sunrise: NaiveTime::from_hms_opt(0, 0, 0)?,
            transit: transit.with_timezone(&Local).time(),
            sunset: NaiveTime::from_hms_opt(23, 59, 59)?,
        }),
        SunriseResult::AllNight { transit: _ } => None,
    }
}

fn current_solar_elevation(lat: f64, lon: f64) -> f64 {
    let now: DateTime<FixedOffset> = Local::now().fixed_offset();
    let delta_t = DeltaT::estimate_from_date_like(now).unwrap_or(69.0);
    spa::solar_position(now, lat, lon, 0.0, delta_t, None)
        .map(|pos| pos.elevation_angle())
        .unwrap_or(0.0)
}

fn day_progress(now: NaiveTime, sunrise: NaiveTime, sunset: NaiveTime) -> f64 {
    if now <= sunrise {
        return 0.0;
    }
    if now >= sunset {
        return 1.0;
    }
    let day_secs = (sunset - sunrise).num_seconds() as f64;
    if day_secs <= 0.0 {
        return 0.0;
    }
    ((now - sunrise).num_seconds() as f64 / day_secs).clamp(0.0, 1.0)
}

#[derive(Debug, Deserialize)]
struct OpenMeteoResponse {
    hourly: OpenMeteoHourly,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoHourly {
    time: Vec<String>,
    cloud_cover: Vec<f64>,
}

fn fetch_weather_forecast(
    lat: f64,
    lon: f64,
    sunrise: NaiveTime,
    sunset: NaiveTime,
) -> Option<Vec<(f64, f64)>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;

    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&hourly=cloud_cover&forecast_days=1&timezone=auto"
    );

    let resp: OpenMeteoResponse = client.get(&url).send().ok()?.json().ok()?;

    let day_secs = (sunset - sunrise).num_seconds() as f64;
    if day_secs <= 0.0 {
        return None;
    }

    let mut forecast = Vec::new();
    for (time_str, &cloud_pct) in resp.hourly.time.iter().zip(resp.hourly.cloud_cover.iter()) {
        let time = time_str
            .split('T')
            .nth(1)
            .and_then(|t| NaiveTime::parse_from_str(t, "%H:%M").ok())?;

        if time < sunrise || time > sunset {
            continue;
        }

        let progress = (time - sunrise).num_seconds() as f64 / day_secs;
        forecast.push((progress.clamp(0.0, 1.0), cloud_pct / 100.0));
    }

    if forecast.is_empty() || forecast[0].0 > 0.01 {
        forecast.insert(0, (0.0, forecast.first().map(|f| f.1).unwrap_or(0.0)));
    }
    if forecast.last().map(|f| f.0).unwrap_or(0.0) < 0.99 {
        forecast.push((1.0, forecast.last().map(|f| f.1).unwrap_or(0.0)));
    }

    Some(forecast)
}

pub fn interpolate_cloud_cover(forecast: &[(f64, f64)], progress: f64) -> f64 {
    if forecast.is_empty() {
        return 0.0;
    }
    if progress <= forecast[0].0 {
        return forecast[0].1;
    }
    if progress >= forecast[forecast.len() - 1].0 {
        return forecast[forecast.len() - 1].1;
    }
    for i in 0..forecast.len() - 1 {
        let (p0, c0) = forecast[i];
        let (p1, c1) = forecast[i + 1];
        if progress >= p0 && progress <= p1 {
            let t = if (p1 - p0).abs() < 1e-12 {
                0.0
            } else {
                (progress - p0) / (p1 - p0)
            };
            return c0 + t * (c1 - c0);
        }
    }
    0.0
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

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

fn check_for_updates(state: &Arc<SharedState>) {
    let client = match reqwest::blocking::Client::builder()
        .user_agent("sunrise-brightness")
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    let api_url = "https://api.github.com/repos/Vianpyro/sunrise-brightness/releases/latest";

    if let Ok(resp) = client.get(api_url).send()
        && let Ok(release) = resp.json::<GitHubRelease>()
    {
        let latest = release.tag_name.trim_start_matches('v').to_string();
        *state.latest_version.write().unwrap() = Some(latest);
    }
}

fn spawn_update_checker(state: Arc<SharedState>) {
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(5));
        check_for_updates(&state);
        loop {
            thread::sleep(Duration::from_secs(4 * 3600));
            check_for_updates(&state);
        }
    });
}

pub fn run_loop(state: Arc<SharedState>) {
    spawn_update_checker(Arc::clone(&state));

    let mut sun_times: Option<SunTimes> = None;
    let mut location: Option<(f64, f64)> = None;

    loop {
        let config = state.config.read().unwrap().clone();

        if state.needs_refetch.swap(false, Ordering::Relaxed)
            || sun_times.is_none()
            || location.is_none()
        {
            let monitors = curve::list_display_names();
            *state.detected_monitors.write().unwrap() = monitors;

            let (lat, lon) = if let (Some(lat), Some(lon)) = (config.latitude, config.longitude) {
                *state.location_str.write().unwrap() = format!("{:.2}°, {:.2}°", lat, lon);
                (lat, lon)
            } else {
                state.set_status("Detecting location...");
                match detect_location() {
                    Some((lat, lon, label)) => {
                        *state.location_str.write().unwrap() = label;
                        (lat, lon)
                    }
                    None => {
                        *state.location_str.write().unwrap() = "Unknown (using defaults)".into();
                        state.set_status("Location detection failed, using 6:00-18:00 defaults");

                        let sr = NaiveTime::from_hms_opt(6, 0, 0).unwrap();
                        let noon = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
                        let ss = NaiveTime::from_hms_opt(18, 0, 0).unwrap();
                        *state.sunrise_str.write().unwrap() = "06:00".into();
                        *state.noon_str.write().unwrap() = "12:00".into();
                        *state.sunset_str.write().unwrap() = "18:00".into();
                        sun_times = Some(SunTimes {
                            sunrise: sr,
                            transit: noon,
                            sunset: ss,
                        });
                        location = None;

                        if let Some(ref st) = sun_times {
                            apply_brightness(&config, &state, st, None);
                        }
                        thread::sleep(Duration::from_secs(config.update_interval_secs));
                        continue;
                    }
                }
            };

            location = Some((lat, lon));

            state.set_status("Computing sun times...");
            match compute_sun_times(lat, lon) {
                Some(st) => {
                    *state.sunrise_str.write().unwrap() = st.sunrise.format("%H:%M").to_string();
                    *state.noon_str.write().unwrap() = st.transit.format("%H:%M").to_string();
                    *state.sunset_str.write().unwrap() = st.sunset.format("%H:%M").to_string();

                    // Fetch weather forecast if enabled
                    if config.weather_adaptive
                        && let Some(forecast) =
                            fetch_weather_forecast(lat, lon, st.sunrise, st.sunset)
                        {
                            *state.weather_forecast.write().unwrap() = forecast;
                        }

                    sun_times = Some(st);
                    state.set_status("Running");
                }
                None => {
                    state.set_status("Polar night — no sunrise today");
                    thread::sleep(Duration::from_secs(config.update_interval_secs));
                    continue;
                }
            }
        }

        if let Some(ref st) = sun_times {
            apply_brightness(&config, &state, st, location);
        }

        thread::sleep(Duration::from_secs(config.update_interval_secs));
    }
}

fn apply_brightness(
    config: &crate::config::Config,
    state: &Arc<SharedState>,
    st: &SunTimes,
    location: Option<(f64, f64)>,
) {
    let now = Local::now().time();
    let progress = day_progress(now, st.sunrise, st.sunset);

    let elevation_deg = location
        .map(|(lat, lon)| current_solar_elevation(lat, lon))
        .unwrap_or(0.0);
    let elevation_norm = (elevation_deg / 90.0).clamp(0.0, 1.0);

    *state.current_elevation.write().unwrap() = elevation_norm;
    *state.current_day_progress.write().unwrap() = progress;

    let weather_factor = if config.weather_adaptive {
        let forecast = state.weather_forecast.read().unwrap();
        let cloud = interpolate_cloud_cover(&forecast, progress);
        *state.current_cloud_cover.write().unwrap() = cloud;
        1.0 - cloud * config.cloud_attenuation
    } else {
        1.0
    };

    let base_brightness = config.global_curve.evaluate(progress);
    let global_target = (base_brightness * weather_factor).clamp(0.0, 100.0) as u32;

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
                Some(m) => m
                    .evaluate(progress, &config.global_curve)
                    .map(|v| (v * weather_factor).clamp(0.0, 100.0)),
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
