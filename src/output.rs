use std::io::Write;

use anyhow::Result;

use crate::Report;

#[derive(Debug, Clone, Copy)]
pub enum OutputMode {
    Compact,
    CompactWithRgb,
    Verbose,
    Hex,
}

pub fn write_json_report(mut writer: impl Write, report: &Report) -> Result<()> {
    serde_json::to_writer_pretty(&mut writer, report)?;
    writeln!(writer)?;
    Ok(())
}

pub fn write_svg_report(mut writer: impl Write, report: &Report) -> Result<()> {
    const SWATCH_WIDTH: u32 = 96;
    const SWATCH_HEIGHT: u32 = 64;
    const BORDER_OPACITY: f32 = 0.08;
    const DIVIDER_OPACITY: f32 = 0.12;

    let swatch_count = report.colors.len().max(1) as u32;
    let width = swatch_count * SWATCH_WIDTH;
    let height = SWATCH_HEIGHT;
    let title = format!("{} palette", report.image.source);
    let desc = format!(
        "{} dominant colors extracted by {} {}",
        report.colors.len(),
        report.tool,
        report.version
    );

    writeln!(
        writer,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}" role="img" aria-labelledby="title desc">"#
    )?;
    writeln!(
        writer,
        "  <title id=\"title\">{}</title>",
        escape_xml(&title)
    )?;
    writeln!(writer, "  <desc id=\"desc\">{}</desc>", escape_xml(&desc))?;

    for (index, color) in report.colors.iter().enumerate() {
        let x = index as u32 * SWATCH_WIDTH;
        let label = format!("{} {:.2}%", color.hex, color.percentage * 100.0);
        writeln!(
            writer,
            r#"  <rect x="{x}" y="0" width="{SWATCH_WIDTH}" height="{height}" fill="{}">"#,
            color.hex
        )?;
        writeln!(writer, "    <title>{}</title>", escape_xml(&label))?;
        writeln!(writer, "  </rect>")?;
    }

    for index in 1..report.colors.len() {
        let x = index as u32 * SWATCH_WIDTH;
        writeln!(
            writer,
            r##"  <line x1="{x}" y1="0" x2="{x}" y2="{height}" stroke="#000000" stroke-opacity="{DIVIDER_OPACITY}"/>"##
        )?;
    }

    writeln!(
        writer,
        r##"  <rect x="0.5" y="0.5" width="{}" height="{}" fill="none" stroke="#000000" stroke-opacity="{BORDER_OPACITY}"/>"##,
        width - 1,
        height - 1
    )?;
    writeln!(writer, "</svg>")?;

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
                if let Some(harmonies) = &color.harmonies {
                    for set in harmonies {
                        writeln!(writer, "      {}", set.harmony)?;
                        for hc in &set.colors {
                            writeln!(
                                writer,
                                "        {}  {}",
                                swatch(hc.rgb.r, hc.rgb.g, hc.rgb.b),
                                hc.hex,
                            )?;
                        }
                    }
                }
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
                if let Some(harmonies) = &color.harmonies {
                    for set in harmonies {
                        writeln!(writer, "      {}", set.harmony)?;
                        for hc in &set.colors {
                            writeln!(
                                writer,
                                "        {}  {:<8} rgb({:>3}, {:>3}, {:>3})",
                                swatch(hc.rgb.r, hc.rgb.g, hc.rgb.b),
                                hc.hex,
                                hc.rgb.r,
                                hc.rgb.g,
                                hc.rgb.b,
                            )?;
                        }
                    }
                }
            }
        }
        OutputMode::Hex => {
            for color in &report.colors {
                writeln!(writer, "{}", color.hex)?;
                if let Some(harmonies) = &color.harmonies {
                    for set in harmonies {
                        for hc in &set.colors {
                            writeln!(writer, "{}", hc.hex)?;
                        }
                    }
                }
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
                if let Some(harmonies) = &color.harmonies {
                    for set in harmonies {
                        writeln!(writer, "      {}", set.harmony)?;
                        for hc in &set.colors {
                            writeln!(
                                writer,
                                "        {}  {:<8}  Oklab({:.4}, {:.4}, {:.4})  {:+.0}°",
                                swatch(hc.rgb.r, hc.rgb.g, hc.rgb.b),
                                hc.hex,
                                hc.oklab.l,
                                hc.oklab.a,
                                hc.oklab.b,
                                hc.hue_offset_degrees,
                            )?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn swatch(r: u8, g: u8, b: u8) -> String {
    format!("\x1b[48;2;{r};{g};{b}m  \x1b[0m")
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{rgb8_to_oklab, Rgb8};
    use crate::{ColorReport, ImageReport, Report, SettingsReport};

    fn report_with_palette() -> Report {
        let first = Rgb8 { r: 255, g: 0, b: 0 };
        let second = Rgb8 {
            r: 0,
            g: 169,
            b: 219,
        };

        Report {
            tool: "huex",
            version: "0.3.0",
            image: ImageReport {
                source: "fixtures/example & test.ppm".into(),
                width: 2,
                height: 1,
                visible_pixels: 2,
                sampled_pixels: 2,
            },
            settings: SettingsReport {
                requested_colors: 2,
                max_iterations: 50,
                sample_limit: 10_000,
                seed: 42,
                color_space: "Oklab",
                initialization: "kmeans++",
                convergence_delta_e: 0.001,
                dedupe_delta_e: 5.0,
                sort: "population".into(),
                harmony: None,
            },
            colors: vec![
                ColorReport {
                    rank: 1,
                    hex: first.hex(),
                    rgb: first,
                    oklab: rgb8_to_oklab(first),
                    population: 1,
                    percentage: 0.5,
                    harmonies: None,
                },
                ColorReport {
                    rank: 2,
                    hex: second.hex(),
                    rgb: second,
                    oklab: rgb8_to_oklab(second),
                    population: 1,
                    percentage: 0.5,
                    harmonies: None,
                },
            ],
        }
    }

    #[test]
    fn svg_output_renders_a_swatch_strip() {
        let mut output = Vec::new();
        let report = report_with_palette();

        write_svg_report(&mut output, &report).unwrap();

        let rendered = String::from_utf8(output).unwrap();
        assert!(rendered.starts_with(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 192 64" width="192" height="64" role="img" aria-labelledby="title desc">"#
        ));
        assert!(rendered
            .contains("<title id=\"title\">fixtures/example &amp; test.ppm palette</title>"));
        assert!(rendered.contains(r##"<rect x="0" y="0" width="96" height="64" fill="#FF0000">"##));
        assert!(rendered.contains(r##"<rect x="96" y="0" width="96" height="64" fill="#00A9DB">"##));
        assert!(rendered.contains(
            r##"<line x1="96" y1="0" x2="96" y2="64" stroke="#000000" stroke-opacity="0.12"/>"##
        ));
    }

    #[test]
    fn svg_output_includes_per_swatch_titles() {
        let mut output = Vec::new();
        let report = report_with_palette();

        write_svg_report(&mut output, &report).unwrap();

        let rendered = String::from_utf8(output).unwrap();
        assert!(rendered.contains("<title>#FF0000 50.00%</title>"));
        assert!(rendered.contains("<title>#00A9DB 50.00%</title>"));
    }
}
