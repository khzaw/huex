use std::fmt;

use clap::ValueEnum;
use serde::Serialize;

use crate::color::{Lab, Rgb8, rgb8_to_oklab};

#[derive(Debug, Clone, Copy, ValueEnum, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HarmonyMode {
    Complement,
    Analogous,
    Triadic,
    SplitComplement,
    Tetradic,
    All,
}

impl fmt::Display for HarmonyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Complement => write!(f, "complement"),
            Self::Analogous => write!(f, "analogous"),
            Self::Triadic => write!(f, "triadic"),
            Self::SplitComplement => write!(f, "split-complement"),
            Self::Tetradic => write!(f, "tetradic"),
            Self::All => write!(f, "all"),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HarmonySet {
    pub harmony: HarmonyMode,
    pub colors: Vec<HarmonyColor>,
}

#[derive(Debug, Serialize)]
pub struct HarmonyColor {
    pub hex: String,
    pub rgb: Rgb8,
    pub oklab: Lab,
    pub hue_offset_degrees: f64,
}

pub fn rotate_hue(lab: Lab, degrees: f64) -> Lab {
    let chroma = (lab.a * lab.a + lab.b * lab.b).sqrt();
    if chroma < 1e-10 {
        return lab;
    }
    let hue = lab.b.atan2(lab.a);
    let new_hue = hue + degrees.to_radians();
    Lab {
        l: lab.l,
        a: chroma * new_hue.cos(),
        b: chroma * new_hue.sin(),
    }
}

fn offsets(mode: HarmonyMode) -> &'static [f64] {
    match mode {
        HarmonyMode::Complement => &[180.0],
        HarmonyMode::Analogous => &[-30.0, 30.0],
        HarmonyMode::Triadic => &[120.0, 240.0],
        HarmonyMode::SplitComplement => &[150.0, 210.0],
        HarmonyMode::Tetradic => &[90.0, 180.0, 270.0],
        HarmonyMode::All => unreachable!(),
    }
}

fn compute_single(lab: Lab, mode: HarmonyMode) -> HarmonySet {
    let colors = offsets(mode)
        .iter()
        .map(|&deg| {
            let rotated = rotate_hue(lab, deg);
            let rgb = rotated.to_rgb8();
            let realized = rgb8_to_oklab(rgb);
            HarmonyColor {
                hex: rgb.hex(),
                rgb,
                oklab: realized,
                hue_offset_degrees: deg,
            }
        })
        .collect();

    HarmonySet {
        harmony: mode,
        colors,
    }
}

const ALL_MODES: [HarmonyMode; 5] = [
    HarmonyMode::Complement,
    HarmonyMode::Analogous,
    HarmonyMode::Triadic,
    HarmonyMode::SplitComplement,
    HarmonyMode::Tetradic,
];

pub fn compute_harmonies(lab: Lab, mode: HarmonyMode) -> Vec<HarmonySet> {
    match mode {
        HarmonyMode::All => ALL_MODES.iter().map(|&m| compute_single(lab, m)).collect(),
        _ => vec![compute_single(lab, mode)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::rgb8_to_oklab;

    #[test]
    fn rotate_180_reverses_hue() {
        let lab = Lab {
            l: 0.5,
            a: 0.1,
            b: 0.0,
        };
        let rotated = rotate_hue(lab, 180.0);
        assert!((rotated.l - 0.5).abs() < 1e-10);
        assert!((rotated.a + 0.1).abs() < 1e-10);
        assert!(rotated.b.abs() < 1e-10);
    }

    #[test]
    fn rotate_360_returns_to_original() {
        let lab = Lab {
            l: 0.7,
            a: 0.05,
            b: -0.08,
        };
        let rotated = rotate_hue(lab, 360.0);
        assert!((rotated.a - lab.a).abs() < 1e-10);
        assert!((rotated.b - lab.b).abs() < 1e-10);
    }

    #[test]
    fn achromatic_unchanged() {
        let lab = Lab {
            l: 0.5,
            a: 0.0,
            b: 0.0,
        };
        let rotated = rotate_hue(lab, 90.0);
        assert_eq!(rotated, lab);
    }

    #[test]
    fn preserves_lightness_and_chroma() {
        let lab = Lab {
            l: 0.7,
            a: 0.1,
            b: 0.05,
        };
        let chroma = (lab.a * lab.a + lab.b * lab.b).sqrt();
        let rotated = rotate_hue(lab, 120.0);
        let rotated_chroma = (rotated.a * rotated.a + rotated.b * rotated.b).sqrt();
        assert!((rotated.l - lab.l).abs() < 1e-10);
        assert!((rotated_chroma - chroma).abs() < 1e-10);
    }

    #[test]
    fn complement_produces_one_color() {
        let lab = Lab {
            l: 0.5,
            a: 0.1,
            b: 0.0,
        };
        let sets = compute_harmonies(lab, HarmonyMode::Complement);
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].colors.len(), 1);
        assert_eq!(sets[0].colors[0].hue_offset_degrees, 180.0);
    }

    #[test]
    fn analogous_produces_two_colors() {
        let lab = Lab {
            l: 0.5,
            a: 0.1,
            b: 0.0,
        };
        let sets = compute_harmonies(lab, HarmonyMode::Analogous);
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].colors.len(), 2);
    }

    #[test]
    fn triadic_produces_two_colors() {
        let lab = Lab {
            l: 0.5,
            a: 0.1,
            b: 0.0,
        };
        let sets = compute_harmonies(lab, HarmonyMode::Triadic);
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].colors.len(), 2);
    }

    #[test]
    fn tetradic_produces_three_colors() {
        let lab = Lab {
            l: 0.5,
            a: 0.1,
            b: 0.0,
        };
        let sets = compute_harmonies(lab, HarmonyMode::Tetradic);
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].colors.len(), 3);
    }

    #[test]
    fn all_produces_five_sets() {
        let lab = Lab {
            l: 0.5,
            a: 0.1,
            b: 0.0,
        };
        let sets = compute_harmonies(lab, HarmonyMode::All);
        assert_eq!(sets.len(), 5);
    }

    #[test]
    fn clipped_harmonies_report_realized_oklab() {
        let lab = rgb8_to_oklab(Rgb8 { r: 0, g: 255, b: 0 });
        let sets = compute_harmonies(lab, HarmonyMode::Complement);
        let harmony = &sets[0].colors[0];
        let realized = rgb8_to_oklab(harmony.rgb);

        assert!((harmony.oklab.l - realized.l).abs() < 1e-10);
        assert!((harmony.oklab.a - realized.a).abs() < 1e-10);
        assert!((harmony.oklab.b - realized.b).abs() < 1e-10);
    }
}
