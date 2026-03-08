mod cluster;
mod color;
mod output;

use std::io::{self, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;
use image::io::Reader as ImageReader;
use serde::Serialize;

use crate::cluster::{
    Cluster, fit_kmeans, merge_close_clusters, nearest_cluster_index, sample_pixels,
};
use crate::color::{Lab, Rgb8, rgb8_to_oklab};
use crate::output::{OutputMode, print_report, write_json_report};

const CONVERGENCE_DELTA_E: f64 = 0.001;
const MERGE_DELTA_E: f64 = 5.0;
const MERGE_OKLAB_DISTANCE: f64 = MERGE_DELTA_E / 100.0;

#[derive(Debug, Parser)]
#[command(
    name = "huex",
    version,
    about = "Extract dominant colors from an image using perceptual Oklab clustering."
)]
pub struct Cli {
    #[arg(
        long,
        value_name = "PATH",
        help = "Path to the image. Use - to read image bytes from stdin."
    )]
    pub image: Option<PathBuf>,

    #[arg(
        value_name = "IMAGE",
        conflicts_with = "image",
        help = "Optional positional image path. Use - to read image bytes from stdin."
    )]
    pub input: Option<PathBuf>,

    #[arg(
        short = 'k',
        long,
        default_value_t = 5,
        help = "Requested number of dominant colors before deduplication."
    )]
    pub k: usize,

    #[arg(
        long = "iter",
        default_value_t = 50,
        help = "Maximum number of k-means iterations."
    )]
    pub max_iterations: usize,

    #[arg(
        long,
        default_value_t = 10_000,
        help = "Maximum number of sampled pixels used for clustering. Use 0 to cluster all visible pixels."
    )]
    pub sample: usize,

    #[arg(
        long,
        default_value_t = 42,
        help = "Random seed used for sampling and k-means++ initialization."
    )]
    pub seed: u64,

    #[arg(long, help = "Emit structured JSON output for agents and scripts.")]
    pub json: bool,

    #[arg(long, help = "Include RGB values in the compact terminal output.")]
    pub rgb: bool,

    #[arg(
        long,
        conflicts_with = "json",
        help = "Show the detailed terminal report with sampling and Oklab values."
    )]
    pub verbose: bool,
}

#[derive(Debug)]
struct Config {
    image: PathBuf,
    k: usize,
    max_iterations: usize,
    sample_limit: usize,
    seed: u64,
    json: bool,
    rgb: bool,
    verbose: bool,
}

#[derive(Debug)]
struct LoadedImage {
    source: String,
    width: u32,
    height: u32,
    pixels: Vec<Rgb8>,
}

#[derive(Debug, Serialize)]
pub struct Report {
    tool: &'static str,
    version: &'static str,
    image: ImageReport,
    settings: SettingsReport,
    colors: Vec<ColorReport>,
}

#[derive(Debug, Serialize)]
struct ImageReport {
    source: String,
    width: u32,
    height: u32,
    visible_pixels: usize,
    sampled_pixels: usize,
}

#[derive(Debug, Serialize)]
struct SettingsReport {
    requested_colors: usize,
    max_iterations: usize,
    sample_limit: usize,
    seed: u64,
    color_space: &'static str,
    initialization: &'static str,
    convergence_delta_e: f64,
    dedupe_delta_e: f64,
}

#[derive(Debug, Serialize)]
pub struct ColorReport {
    rank: usize,
    hex: String,
    rgb: Rgb8,
    oklab: Lab,
    population: usize,
    percentage: f64,
}

impl Config {
    fn from_cli(cli: Cli) -> Result<Self> {
        let image = cli.image.or(cli.input).ok_or_else(|| {
            anyhow!("missing image path; pass --image <PATH> or a positional IMAGE")
        })?;

        if cli.k == 0 {
            bail!("--k must be greater than 0");
        }

        if cli.max_iterations == 0 {
            bail!("--iter must be greater than 0");
        }

        Ok(Self {
            image,
            k: cli.k,
            max_iterations: cli.max_iterations,
            sample_limit: cli.sample,
            seed: cli.seed,
            json: cli.json,
            rgb: cli.rgb,
            verbose: cli.verbose,
        })
    }
}

pub fn run(cli: Cli) -> Result<()> {
    let config = Config::from_cli(cli)?;
    let report = analyze(&config)?;

    if config.json {
        write_json_report(io::stdout(), &report)?;
    } else {
        let mode = if config.verbose {
            OutputMode::Verbose
        } else if config.rgb {
            OutputMode::CompactWithRgb
        } else {
            OutputMode::Compact
        };
        print_report(io::stdout(), &report, mode)?;
    }

    Ok(())
}

fn analyze(config: &Config) -> Result<Report> {
    let image = load_image(&config.image)?;
    let sampled_pixels = sample_pixels(&image.pixels, config.sample_limit, config.seed);
    let sampled_points: Vec<Lab> = sampled_pixels
        .iter()
        .map(|pixel| rgb8_to_oklab(*pixel))
        .collect();

    let raw_clusters = fit_kmeans(
        &sampled_points,
        config.k,
        config.max_iterations,
        config.seed,
        CONVERGENCE_DELTA_E,
    )?;
    let merged_clusters = merge_close_clusters(raw_clusters, MERGE_OKLAB_DISTANCE);
    let colors = summarize_colors(&image.pixels, &merged_clusters);

    Ok(Report {
        tool: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        image: ImageReport {
            source: image.source,
            width: image.width,
            height: image.height,
            visible_pixels: image.pixels.len(),
            sampled_pixels: sampled_points.len(),
        },
        settings: SettingsReport {
            requested_colors: config.k,
            max_iterations: config.max_iterations,
            sample_limit: config.sample_limit,
            seed: config.seed,
            color_space: "Oklab",
            initialization: "kmeans++",
            convergence_delta_e: CONVERGENCE_DELTA_E,
            dedupe_delta_e: MERGE_DELTA_E,
        },
        colors,
    })
}

fn load_image(path: &Path) -> Result<LoadedImage> {
    let source = path.display().to_string();
    let dynamic = if source == "-" {
        let mut bytes = Vec::new();
        io::stdin()
            .read_to_end(&mut bytes)
            .context("failed to read image bytes from stdin")?;
        if bytes.is_empty() {
            bail!("stdin did not contain any image bytes");
        }
        image::load_from_memory(&bytes).context("failed to decode image bytes from stdin")?
    } else {
        ImageReader::open(path)
            .with_context(|| format!("failed to open image at {}", path.display()))?
            .decode()
            .with_context(|| format!("failed to decode image at {}", path.display()))?
    };

    let width = dynamic.width();
    let height = dynamic.height();
    let rgba = dynamic.to_rgba8();
    let pixels: Vec<Rgb8> = rgba
        .pixels()
        .filter_map(|pixel| blend_over_white(pixel.0))
        .collect();

    if pixels.is_empty() {
        bail!("image has no visible pixels after transparency handling");
    }

    Ok(LoadedImage {
        source,
        width,
        height,
        pixels,
    })
}

fn blend_over_white(pixel: [u8; 4]) -> Option<Rgb8> {
    let alpha = pixel[3] as f64 / 255.0;
    if alpha <= f64::EPSILON {
        return None;
    }

    let blend =
        |channel: u8| -> u8 { ((channel as f64 * alpha) + (255.0 * (1.0 - alpha))).round() as u8 };

    Some(Rgb8 {
        r: blend(pixel[0]),
        g: blend(pixel[1]),
        b: blend(pixel[2]),
    })
}

fn summarize_colors(pixels: &[Rgb8], clusters: &[Cluster]) -> Vec<ColorReport> {
    let mut counts = vec![0usize; clusters.len()];
    let mut sums = vec![Lab::zero(); clusters.len()];

    for pixel in pixels {
        let lab = rgb8_to_oklab(*pixel);
        let index = nearest_cluster_index(lab, clusters);
        counts[index] += 1;
        sums[index] += lab;
    }

    let total_pixels = pixels.len() as f64;
    let mut colors = Vec::new();

    for index in 0..clusters.len() {
        if counts[index] == 0 {
            continue;
        }

        let centroid = sums[index] / counts[index] as f64;
        let rgb = centroid.to_rgb8();
        colors.push(ColorReport {
            rank: 0,
            hex: rgb.hex(),
            rgb,
            oklab: centroid,
            population: counts[index],
            percentage: counts[index] as f64 / total_pixels,
        });
    }

    colors.sort_by(|left, right| right.population.cmp(&left.population));
    for (index, color) in colors.iter_mut().enumerate() {
        color.rank = index + 1;
    }

    colors
}
