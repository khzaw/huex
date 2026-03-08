use serde::Serialize;
use std::ops::{Add, AddAssign, Div, Mul};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Rgb8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Lab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

impl Rgb8 {
    pub fn hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

impl Lab {
    pub fn zero() -> Self {
        Self {
            l: 0.0,
            a: 0.0,
            b: 0.0,
        }
    }

    pub fn distance(self, other: Self) -> f64 {
        self.distance_squared(other).sqrt()
    }

    pub fn distance_squared(self, other: Self) -> f64 {
        let dl = self.l - other.l;
        let da = self.a - other.a;
        let db = self.b - other.b;
        (dl * dl) + (da * da) + (db * db)
    }

    pub fn to_rgb8(self) -> Rgb8 {
        oklab_to_rgb8(self)
    }
}

impl Add for Lab {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            l: self.l + rhs.l,
            a: self.a + rhs.a,
            b: self.b + rhs.b,
        }
    }
}

impl AddAssign for Lab {
    fn add_assign(&mut self, rhs: Self) {
        self.l += rhs.l;
        self.a += rhs.a;
        self.b += rhs.b;
    }
}

impl Mul<f64> for Lab {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            l: self.l * rhs,
            a: self.a * rhs,
            b: self.b * rhs,
        }
    }
}

impl Div<f64> for Lab {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self {
            l: self.l / rhs,
            a: self.a / rhs,
            b: self.b / rhs,
        }
    }
}

pub fn rgb8_to_oklab(rgb: Rgb8) -> Lab {
    let r = srgb_to_linear(rgb.r as f64 / 255.0);
    let g = srgb_to_linear(rgb.g as f64 / 255.0);
    let b = srgb_to_linear(rgb.b as f64 / 255.0);

    let l = 0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b;
    let m = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
    let s = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;

    let l_root = l.cbrt();
    let m_root = m.cbrt();
    let s_root = s.cbrt();

    Lab {
        l: 0.210_454_255_3 * l_root + 0.793_617_785 * m_root - 0.004_072_046_8 * s_root,
        a: 1.977_998_495_1 * l_root - 2.428_592_205 * m_root + 0.450_593_709_9 * s_root,
        b: 0.025_904_037_1 * l_root + 0.782_771_766_2 * m_root - 0.808_675_766 * s_root,
    }
}

fn oklab_to_rgb8(lab: Lab) -> Rgb8 {
    let l_root = lab.l + 0.396_337_777_4 * lab.a + 0.215_803_757_3 * lab.b;
    let m_root = lab.l - 0.105_561_345_8 * lab.a - 0.063_854_172_8 * lab.b;
    let s_root = lab.l - 0.089_484_177_5 * lab.a - 1.291_485_548 * lab.b;

    let l = l_root * l_root * l_root;
    let m = m_root * m_root * m_root;
    let s = s_root * s_root * s_root;

    let r = 4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s;
    let g = -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s;
    let b = -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701 * s;

    Rgb8 {
        r: linear_to_rgb8(r),
        g: linear_to_rgb8(g),
        b: linear_to_rgb8(b),
    }
}

fn srgb_to_linear(value: f64) -> f64 {
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f64) -> f64 {
    let clamped = value.clamp(0.0, 1.0);
    if clamped <= 0.003_130_8 {
        clamped * 12.92
    } else {
        1.055 * clamped.powf(1.0 / 2.4) - 0.055
    }
}

fn linear_to_rgb8(value: f64) -> u8 {
    (linear_to_srgb(value) * 255.0).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::{Lab, Rgb8, rgb8_to_oklab};

    #[test]
    fn rgb_roundtrip_stays_close() {
        let samples = [
            Rgb8 { r: 255, g: 0, b: 0 },
            Rgb8 { r: 0, g: 255, b: 0 },
            Rgb8 { r: 0, g: 0, b: 255 },
            Rgb8 {
                r: 12,
                g: 34,
                b: 56,
            },
            Rgb8 {
                r: 220,
                g: 200,
                b: 18,
            },
        ];

        for sample in samples {
            let roundtrip = rgb8_to_oklab(sample).to_rgb8();
            assert!((sample.r as i16 - roundtrip.r as i16).abs() <= 1);
            assert!((sample.g as i16 - roundtrip.g as i16).abs() <= 1);
            assert!((sample.b as i16 - roundtrip.b as i16).abs() <= 1);
        }
    }

    #[test]
    fn lab_distance_is_zero_for_same_point() {
        let point = Lab {
            l: 0.5,
            a: -0.1,
            b: 0.2,
        };
        assert_eq!(point.distance(point), 0.0);
    }
}
