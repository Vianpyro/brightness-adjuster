use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ControlPoint {
    #[serde(alias = "elevation")]
    pub position: f64,

    pub brightness: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BrightnessCurve {
    pub points: Vec<ControlPoint>,
    #[serde(default)]
    pub preset: Option<String>,
}

impl Default for BrightnessCurve {
    fn default() -> Self {
        Self::natural()
    }
}

impl BrightnessCurve {
    pub fn natural() -> Self {
        Self {
            points: vec![
                ControlPoint {
                    position: 0.00,
                    brightness: 0.0,
                },
                ControlPoint {
                    position: 0.10,
                    brightness: 50.0,
                },
                ControlPoint {
                    position: 0.25,
                    brightness: 80.0,
                },
                ControlPoint {
                    position: 0.50,
                    brightness: 100.0,
                },
                ControlPoint {
                    position: 0.75,
                    brightness: 80.0,
                },
                ControlPoint {
                    position: 0.90,
                    brightness: 50.0,
                },
                ControlPoint {
                    position: 1.00,
                    brightness: 0.0,
                },
            ],
            preset: Some("natural".into()),
        }
    }

    pub fn linear() -> Self {
        Self {
            points: vec![
                ControlPoint {
                    position: 0.0,
                    brightness: 0.0,
                },
                ControlPoint {
                    position: 0.5,
                    brightness: 100.0,
                },
                ControlPoint {
                    position: 1.0,
                    brightness: 0.0,
                },
            ],
            preset: Some("linear".into()),
        }
    }

    pub fn night_owl() -> Self {
        Self {
            points: vec![
                ControlPoint {
                    position: 0.00,
                    brightness: 5.0,
                },
                ControlPoint {
                    position: 0.25,
                    brightness: 30.0,
                },
                ControlPoint {
                    position: 0.50,
                    brightness: 50.0,
                },
                ControlPoint {
                    position: 0.75,
                    brightness: 35.0,
                },
                ControlPoint {
                    position: 1.00,
                    brightness: 5.0,
                },
            ],
            preset: Some("night_owl".into()),
        }
    }

    pub fn from_preset(name: &str) -> Self {
        match name {
            "linear" => Self::linear(),
            "night_owl" => Self::night_owl(),
            _ => Self::natural(),
        }
    }
}

impl BrightnessCurve {
    pub fn sort(&mut self) {
        self.points.sort_by(|a, b| {
            a.position
                .partial_cmp(&b.position)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    pub fn add_point(&mut self, position: f64, brightness: f64) -> bool {
        if self.points.len() >= 10 {
            return false;
        }
        self.points.push(ControlPoint {
            position,
            brightness,
        });
        self.sort();
        self.preset = None;
        true
    }

    pub fn remove_point(&mut self, index: usize) -> bool {
        if index == 0 || index >= self.points.len().saturating_sub(1) || self.points.len() <= 2 {
            return false;
        }
        self.points.remove(index);
        self.preset = None;
        true
    }
}

impl BrightnessCurve {
    pub fn evaluate(&self, x: f64) -> f64 {
        let pts = &self.points;
        let n = pts.len();
        if n == 0 {
            return 50.0;
        }
        if n == 1 {
            return pts[0].brightness;
        }

        let x = x.clamp(0.0, 1.0);

        if x <= pts[0].position {
            return pts[0].brightness;
        }
        if x >= pts[n - 1].position {
            return pts[n - 1].brightness;
        }

        let mut k = 0;
        for i in 0..n - 1 {
            if x >= pts[i].position && x <= pts[i + 1].position {
                k = i;
                break;
            }
        }

        let mut delta = vec![0.0_f64; n - 1];
        for i in 0..n - 1 {
            let dx = pts[i + 1].position - pts[i].position;
            delta[i] = if dx.abs() < 1e-12 {
                0.0
            } else {
                (pts[i + 1].brightness - pts[i].brightness) / dx
            };
        }

        let mut m = vec![0.0_f64; n];
        m[0] = delta[0];
        m[n - 1] = delta[n - 2];
        for i in 1..n - 1 {
            if delta[i - 1].signum() != delta[i].signum() {
                m[i] = 0.0;
            } else {
                m[i] = (delta[i - 1] + delta[i]) / 2.0;
            }
        }

        for i in 0..n - 1 {
            if delta[i].abs() < 1e-12 {
                m[i] = 0.0;
                m[i + 1] = 0.0;
            } else {
                let alpha = m[i] / delta[i];
                let beta = m[i + 1] / delta[i];
                let tau = alpha * alpha + beta * beta;
                if tau > 9.0 {
                    let s = 3.0 / tau.sqrt();
                    m[i] = s * alpha * delta[i];
                    m[i + 1] = s * beta * delta[i];
                }
            }
        }

        let h = pts[k + 1].position - pts[k].position;
        if h.abs() < 1e-12 {
            return pts[k].brightness;
        }
        let t = (x - pts[k].position) / h;
        let t2 = t * t;
        let t3 = t2 * t;

        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        let result = h00 * pts[k].brightness
            + h10 * h * m[k]
            + h01 * pts[k + 1].brightness
            + h11 * h * m[k + 1];

        result.clamp(0.0, 100.0)
    }
}

pub fn natural_reference(day_progress: f64) -> f64 {
    if day_progress <= 0.0 || day_progress >= 1.0 {
        return 0.0;
    }
    let elevation = (day_progress * std::f64::consts::PI).sin();
    100.0 * elevation.powf(0.45)
}

pub const SNAP_X: &[f64] = &[0.0, 0.25, 0.5, 0.75, 1.0];

pub const SNAP_Y: &[f64] = &[0.0, 25.0, 50.0, 75.0, 100.0];

pub const SNAP_RADIUS: f64 = 0.03;

pub fn snap_to_grid(value: f64, grid: &[f64], axis_range: f64) -> f64 {
    let threshold = SNAP_RADIUS * axis_range;
    for &g in grid {
        if (value - g).abs() <= threshold {
            return g;
        }
    }
    value
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonitorOverride {
    pub name: String,
    #[serde(default)]
    pub alias: Option<String>,

    pub mode: String,
    #[serde(default)]
    pub offset: i32,
    #[serde(default)]
    pub curve: Option<BrightnessCurve>,
}

impl MonitorOverride {
    pub fn new_global(name: String) -> Self {
        Self {
            alias: None,
            name,
            mode: "global".into(),
            offset: 0,
            curve: None,
        }
    }

    pub fn display_label(&self) -> String {
        if let Some(alias) = &self.alias
            && !alias.is_empty()
        {
            return alias.clone();
        }
        clean_display_name(&self.name)
    }

    pub fn evaluate(&self, day_progress: f64, global_curve: &BrightnessCurve) -> Option<f64> {
        match self.mode.as_str() {
            "ignored" => None,
            "offset" => {
                let base = global_curve.evaluate(day_progress);
                Some((base + self.offset as f64).clamp(0.0, 100.0))
            }
            "custom" => Some(self.curve.as_ref().map_or_else(
                || global_curve.evaluate(day_progress),
                |c| c.evaluate(day_progress),
            )),
            _ => Some(global_curve.evaluate(day_progress)),
        }
    }
}

pub fn clean_display_name(raw: &str) -> String {
    let s = raw
        .trim_start_matches("\\\\.\\")
        .trim_start_matches("\\\\?\\");
    let s = s.split('\\').next().unwrap_or(s);
    if let Some(rest) = s.strip_prefix("DISPLAY")
        && let Ok(n) = rest.parse::<u32>()
    {
        return format!("Display {n}");
    }
    s.to_string()
}

pub fn list_display_names() -> Vec<String> {
    use brightness::blocking::{Brightness, brightness_devices};
    brightness_devices()
        .flatten()
        .filter_map(|dev| dev.device_name().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_interpolation() {
        let curve = BrightnessCurve::linear();
        assert!((curve.evaluate(0.0) - 0.0).abs() < 1.0);
        assert!((curve.evaluate(0.25) - 50.0).abs() < 2.0);
        assert!((curve.evaluate(0.5) - 100.0).abs() < 1.0);
        assert!((curve.evaluate(0.75) - 50.0).abs() < 2.0);
        assert!((curve.evaluate(1.0) - 0.0).abs() < 1.0);
    }

    #[test]
    fn natural_preset_is_bell_shaped() {
        let curve = BrightnessCurve::natural();
        let sunrise = curve.evaluate(0.0);
        let morning = curve.evaluate(0.25);
        let zenith = curve.evaluate(0.5);
        let afternoon = curve.evaluate(0.75);
        let sunset = curve.evaluate(1.0);

        assert!(sunrise < 5.0);
        assert!(morning > 50.0);
        assert!(zenith > 95.0);
        assert!(afternoon > 50.0);
        assert!(sunset < 5.0);
    }

    #[test]
    fn never_overshoots() {
        let curve = BrightnessCurve::natural();
        for i in 0..=100 {
            let v = curve.evaluate(i as f64 / 100.0);
            assert!((0.0..=100.0).contains(&v), "overshoot at {i}: {v}");
        }
    }

    #[test]
    fn snap_works() {
        assert!((snap_to_grid(0.24, SNAP_X, 1.0) - 0.25).abs() < 0.001);
        assert!((snap_to_grid(0.26, SNAP_X, 1.0) - 0.25).abs() < 0.001);
        assert!((snap_to_grid(0.20, SNAP_X, 1.0) - 0.20).abs() < 0.001);
        assert!((snap_to_grid(74.0, SNAP_Y, 100.0) - 75.0).abs() < 0.001);
    }

    #[test]
    fn add_remove_points() {
        let mut curve = BrightnessCurve::linear();
        assert_eq!(curve.points.len(), 3);
        assert!(curve.add_point(0.25, 60.0));
        assert_eq!(curve.points.len(), 4);
        assert!(curve.preset.is_none());
        assert!(!curve.remove_point(0));
        assert!(curve.remove_point(1));
        assert_eq!(curve.points.len(), 3);
    }
}
