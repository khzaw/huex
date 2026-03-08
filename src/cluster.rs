use anyhow::{Result, bail};

use crate::color::{Lab, Rgb8};

#[derive(Debug, Clone)]
pub struct Cluster {
    pub centroid: Lab,
    pub weight: usize,
}

#[derive(Debug, Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.0 = value;
        value.wrapping_mul(2_685_821_657_736_338_717)
    }

    fn next_f64(&mut self) -> f64 {
        let value = self.next_u64() >> 11;
        (value as f64) / ((1u64 << 53) as f64)
    }

    fn index(&mut self, upper_bound: usize) -> usize {
        if upper_bound <= 1 {
            return 0;
        }

        (self.next_u64() as usize) % upper_bound
    }
}

pub fn sample_pixels(pixels: &[Rgb8], sample_limit: usize, seed: u64) -> Vec<Rgb8> {
    if pixels.is_empty() {
        return Vec::new();
    }

    if sample_limit == 0 || sample_limit >= pixels.len() {
        return pixels.to_vec();
    }

    let mut rng = Rng::new(seed);
    let mut reservoir = pixels[..sample_limit].to_vec();

    for (index, pixel) in pixels.iter().enumerate().skip(sample_limit) {
        let slot = rng.index(index + 1);
        if slot < sample_limit {
            reservoir[slot] = *pixel;
        }
    }

    reservoir
}

pub fn fit_kmeans(
    points: &[Lab],
    requested_k: usize,
    max_iterations: usize,
    seed: u64,
    convergence_delta_e: f64,
) -> Result<Vec<Cluster>> {
    if points.is_empty() {
        bail!("cannot cluster an empty point set");
    }

    let k = requested_k.min(points.len()).max(1);
    let mut centroids = init_kmeans_plus_plus(points, k, seed);
    let mut assignments = vec![0usize; points.len()];
    let mut sums = vec![Lab::zero(); centroids.len()];
    let mut counts = vec![0usize; centroids.len()];

    for _ in 0..max_iterations {
        sums.iter_mut().for_each(|s| *s = Lab::zero());
        counts.iter_mut().for_each(|c| *c = 0);
        let mut changed = false;

        for (index, point) in points.iter().copied().enumerate() {
            let nearest = nearest_index(point, &centroids);
            if assignments[index] != nearest {
                changed = true;
                assignments[index] = nearest;
            }
            sums[nearest] += point;
            counts[nearest] += 1;
        }

        let mut max_shift = 0.0_f64;
        for centroid_index in 0..centroids.len() {
            if counts[centroid_index] == 0 {
                centroids[centroid_index] = farthest_point(points, &centroids);
                changed = true;
                continue;
            }

            let updated = sums[centroid_index] / counts[centroid_index] as f64;
            max_shift = max_shift.max(centroids[centroid_index].distance(updated));
            centroids[centroid_index] = updated;
        }

        if !changed || max_shift < convergence_delta_e {
            break;
        }
    }

    // Final pass: recompute sums/counts against the latest centroids so
    // cluster centroids and weights are consistent when the loop exhausted
    // max_iterations without converging.
    sums.iter_mut().for_each(|s| *s = Lab::zero());
    counts.iter_mut().for_each(|c| *c = 0);
    for point in points.iter().copied() {
        let nearest = nearest_index(point, &centroids);
        sums[nearest] += point;
        counts[nearest] += 1;
    }

    let mut clusters = Vec::new();
    for centroid_index in 0..centroids.len() {
        if counts[centroid_index] == 0 {
            continue;
        }

        clusters.push(Cluster {
            centroid: sums[centroid_index] / counts[centroid_index] as f64,
            weight: counts[centroid_index],
        });
    }

    Ok(clusters)
}

pub fn merge_close_clusters(mut clusters: Vec<Cluster>, threshold: f64) -> Vec<Cluster> {
    if clusters.len() <= 1 {
        return clusters;
    }

    loop {
        let mut best_pair = None;
        let mut best_distance = f64::MAX;

        for left in 0..clusters.len() {
            for right in (left + 1)..clusters.len() {
                let distance = clusters[left].centroid.distance(clusters[right].centroid);
                if distance < threshold && distance < best_distance {
                    best_distance = distance;
                    best_pair = Some((left, right));
                }
            }
        }

        let Some((left, right)) = best_pair else {
            break;
        };

        let left_weight = clusters[left].weight as f64;
        let right_weight = clusters[right].weight as f64;
        let total_weight = left_weight + right_weight;
        let merged = Cluster {
            centroid: (clusters[left].centroid * left_weight
                + clusters[right].centroid * right_weight)
                / total_weight,
            weight: clusters[left].weight + clusters[right].weight,
        };

        clusters[left] = merged;
        clusters.remove(right);
    }

    clusters.sort_by(|left, right| right.weight.cmp(&left.weight));
    clusters
}

pub fn nearest_cluster_index(point: Lab, clusters: &[Cluster]) -> usize {
    let mut best_index = 0usize;
    let mut best_distance = f64::MAX;

    for (index, cluster) in clusters.iter().enumerate() {
        let distance = point.distance_squared(cluster.centroid);
        if distance < best_distance {
            best_distance = distance;
            best_index = index;
        }
    }

    best_index
}

fn nearest_index(point: Lab, centroids: &[Lab]) -> usize {
    let mut best_index = 0usize;
    let mut best_distance = f64::MAX;

    for (index, centroid) in centroids.iter().copied().enumerate() {
        let distance = point.distance_squared(centroid);
        if distance < best_distance {
            best_distance = distance;
            best_index = index;
        }
    }

    best_index
}

fn init_kmeans_plus_plus(points: &[Lab], k: usize, seed: u64) -> Vec<Lab> {
    let mut rng = Rng::new(seed);
    let mut centroids = Vec::with_capacity(k);
    let first = points[rng.index(points.len())];
    centroids.push(first);

    let mut distances: Vec<f64> = points
        .iter()
        .map(|point| point.distance_squared(first))
        .collect();

    while centroids.len() < k {
        let total_distance: f64 = distances.iter().sum();
        if total_distance <= f64::EPSILON {
            centroids.push(points[rng.index(points.len())]);
        } else {
            let mut target = rng.next_f64() * total_distance;
            let mut chosen = points[0];

            for (index, distance) in distances.iter().copied().enumerate() {
                target -= distance;
                if target <= 0.0 {
                    chosen = points[index];
                    break;
                }
            }

            centroids.push(chosen);
        }

        let new_centroid = *centroids.last().unwrap();
        for (dist, point) in distances.iter_mut().zip(points.iter()) {
            let d = point.distance_squared(new_centroid);
            if d < *dist {
                *dist = d;
            }
        }
    }

    centroids
}

fn farthest_point(points: &[Lab], centroids: &[Lab]) -> Lab {
    let mut best_point = points[0];
    let mut best_distance = f64::MIN;

    for point in points.iter().copied() {
        let distance = centroids
            .iter()
            .copied()
            .map(|centroid| point.distance_squared(centroid))
            .fold(f64::MAX, f64::min);
        if distance > best_distance {
            best_distance = distance;
            best_point = point;
        }
    }

    best_point
}

#[cfg(test)]
mod tests {
    use super::{Cluster, fit_kmeans, merge_close_clusters};
    use crate::color::Lab;

    #[test]
    fn merges_close_clusters() {
        let clusters = vec![
            Cluster {
                centroid: Lab {
                    l: 0.6,
                    a: 0.1,
                    b: 0.1,
                },
                weight: 10,
            },
            Cluster {
                centroid: Lab {
                    l: 0.6005,
                    a: 0.1005,
                    b: 0.0995,
                },
                weight: 5,
            },
            Cluster {
                centroid: Lab {
                    l: 0.2,
                    a: -0.1,
                    b: -0.1,
                },
                weight: 3,
            },
        ];

        let merged = merge_close_clusters(clusters, 0.05);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].weight, 15);
    }

    #[test]
    fn fits_two_clear_clusters() {
        let mut points = Vec::new();
        for _ in 0..50 {
            points.push(Lab {
                l: 0.8,
                a: 0.1,
                b: 0.1,
            });
            points.push(Lab {
                l: 0.3,
                a: -0.1,
                b: -0.1,
            });
        }

        let clusters = fit_kmeans(&points, 2, 10, 42, 0.001).unwrap();
        assert_eq!(clusters.len(), 2);
        let total_weight: usize = clusters.iter().map(|cluster| cluster.weight).sum();
        assert_eq!(total_weight, points.len());
    }
}
