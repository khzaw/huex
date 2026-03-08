use std::io::Write;

use anyhow::Result;

use crate::Report;

#[derive(Debug, Clone, Copy)]
pub enum OutputMode {
    Compact,
    CompactWithRgb,
    Verbose,
}

pub fn write_json_report(mut writer: impl Write, report: &Report) -> Result<()> {
    serde_json::to_writer_pretty(&mut writer, report)?;
    writeln!(writer)?;
    Ok(())
}

pub fn print_report(mut writer: impl Write, report: &Report, mode: OutputMode) -> Result<()> {
    match mode {
        OutputMode::Compact => {
            for color in &report.colors {
                writeln!(
                    writer,
                    "  {}  {:<8} {:>6.2}%",
                    swatch(color.rgb.r, color.rgb.g, color.rgb.b),
                    color.hex,
                    color.percentage * 100.0,
                )?;
            }
        }
        OutputMode::CompactWithRgb => {
            for color in &report.colors {
                writeln!(
                    writer,
                    "  {}  {:<8} rgb({:>3}, {:>3}, {:>3})  {:>6.2}%",
                    swatch(color.rgb.r, color.rgb.g, color.rgb.b),
                    color.hex,
                    color.rgb.r,
                    color.rgb.g,
                    color.rgb.b,
                    color.percentage * 100.0,
                )?;
            }
        }
        OutputMode::Verbose => {
            writeln!(
                writer,
                "huex {}  {}  {}x{}  {} visible pixels",
                report.version,
                report.image.source,
                report.image.width,
                report.image.height,
                report.image.visible_pixels
            )?;
            writeln!(
                writer,
                "Oklab k-means++  requested={}  sampled={}  seed={}",
                report.settings.requested_colors, report.image.sampled_pixels, report.settings.seed
            )?;
            writeln!(writer)?;

            for color in &report.colors {
                writeln!(
                    writer,
                    "  {}  {:<8}  rgb({:>3}, {:>3}, {:>3})  {:>6.2}%  {:>8} px  Oklab({:.4}, {:.4}, {:.4})",
                    swatch(color.rgb.r, color.rgb.g, color.rgb.b),
                    color.hex,
                    color.rgb.r,
                    color.rgb.g,
                    color.rgb.b,
                    color.percentage * 100.0,
                    color.population,
                    color.oklab.l,
                    color.oklab.a,
                    color.oklab.b
                )?;
            }
        }
    }

    Ok(())
}

fn swatch(r: u8, g: u8, b: u8) -> String {
    format!("\x1b[48;2;{r};{g};{b}m  \x1b[0m")
}
