// Copyright (C) 2004-2006 Alex Shovkoplyas, VE3NEA (referenced algorithms).
// Copyright (C) 2026 Vladimir Vecgailis (Rust implementation).
// Rust reimplementation of algorithms from QuickAvg.pas, MovAvg.pas,
// VolumCtl.pas, Qsb.pas and Mixers.pas.
// SPDX-License-Identifier: MPL-2.0
use std::f32::consts::{PI, TAU};
pub const RATE: u32 = 11025;
pub const BLOCK: usize = 512;
pub const NEVER: i32 = i32::MAX;
/// The original distributions; explicit RNG state makes scenarios reproducible.
pub struct Random(pub u64);
impl Random {
    pub fn uniform(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (((self.0 >> 40) as f32) + 0.5) / 16777216.
    }
    pub fn int(&mut self, n: usize) -> usize {
        ((self.uniform() * n as f32) as usize).min(n.saturating_sub(1))
    }
    pub fn signed(&mut self) -> f32 {
        2. * self.uniform() - 1.
    }
    pub fn normal(&mut self) -> f32 {
        (-2. * self.uniform().ln()).sqrt() * (TAU * self.uniform()).cos()
    }
    pub fn gauss(&mut self, mean: f32, limit: f32) -> f32 {
        (mean + self.normal() * 0.5 * limit).clamp(mean - limit, mean + limit)
    }
    pub fn rayleigh(&mut self, mean: f32) -> f32 {
        mean * (-self.uniform().ln() - self.uniform().ln()).sqrt()
    }
    pub fn ushaped(&mut self) -> f32 {
        (PI * (self.uniform() - 0.5)).sin()
    }
    pub fn poisson(&mut self, mean: f32) -> usize {
        let g = (-mean).exp();
        let mut t = 1.;
        for n in 0..=30 {
            t *= self.uniform();
            if t <= g {
                return n;
            }
        }
        30
    }
}
pub fn blocks(seconds: f32) -> i32 {
    (RATE as f32 / BLOCK as f32 * seconds).round_ties_even() as i32
}
/// Cascaded boxcars. f64 accumulators follow TQuickAverage and avoid long-run drift.
#[derive(Clone)]
pub struct Average {
    buffers: Vec<Vec<f64>>,
    sums: Vec<f64>,
    index: usize,
    points: usize,
}
impl Average {
    pub fn new(points: usize, passes: usize) -> Self {
        let points = points.max(1);
        Self {
            buffers: vec![vec![0.; points]; passes],
            sums: vec![0.; passes],
            index: 0,
            points,
        }
    }
    pub fn apply(&mut self, value: f32) -> f32 {
        let mut v = value as f64;
        for (buf, sum) in self.buffers.iter_mut().zip(&mut self.sums) {
            *sum += v - buf[self.index];
            buf[self.index] = v;
            v = *sum;
        }
        self.index = (self.index + 1) % self.points;
        (v / (self.points as f64).powi(self.buffers.len() as i32)) as f32
    }
    pub fn reset(&mut self) {
        for b in &mut self.buffers {
            b.fill(0.);
        }
        self.sums.fill(0.);
        self.index = 0;
    }
}
pub struct Fading {
    re: Average,
    im: Average,
    gain: f32,
    step: f32,
    left: usize,
    points: usize,
}
impl Fading {
    pub fn new(bandwidth: f32, rng: &mut Random) -> Self {
        let points = (0.37 * RATE as f32 / (BLOCK as f32 / 4. * bandwidth)).ceil() as usize;
        let mut f = Self {
            re: Average::new(points, 3),
            im: Average::new(points, 3),
            gain: 0.,
            step: 0.,
            left: 0,
            points,
        };
        for _ in 0..=points * 3 {
            f.gain = f.new_gain(rng);
        }
        f
    }
    pub fn set_bandwidth(&mut self, bandwidth: f32, rng: &mut Random) {
        *self = Self::new(bandwidth, rng);
    }
    fn new_gain(&mut self, rng: &mut Random) -> f32 {
        let re = self.re.apply(rng.signed());
        let im = self.im.apply(rng.signed());
        ((re * re + im * im) * 3. * self.points as f32).sqrt()
    }
    pub fn apply(&mut self, env: &mut [f32], rng: &mut Random) {
        for v in env {
            if self.left == 0 {
                self.step = (self.new_gain(rng) - self.gain) / (BLOCK / 4) as f32;
                self.left = BLOCK / 4;
            }
            *v *= self.gain;
            self.gain += self.step;
            self.left -= 1;
        }
    }
}
/// Original paired three-pass complex LPFs, including warm-filter reset/swap.
pub struct Receiver {
    re: Average,
    im: Average,
    warm_re: Average,
    warm_im: Average,
    bw: f32,
    block: usize,
}
impl Receiver {
    pub fn new(bw: f32) -> Self {
        let n = (0.7 * RATE as f32 / bw).round_ties_even() as usize;
        Self {
            re: Average::new(n, 3),
            im: Average::new(n, 3),
            warm_re: Average::new(n, 3),
            warm_im: Average::new(n, 3),
            bw,
            block: 0,
        }
    }
    pub fn apply(&mut self, re: &mut [f32], im: &mut [f32], bw: f32) {
        if self.bw != bw {
            *self = Self::new(bw);
        }
        let gain = (500. / bw).sqrt();
        for (r, i) in re.iter_mut().zip(im) {
            self.warm_re.apply(*r);
            self.warm_im.apply(*i);
            *r = self.re.apply(*r) * gain;
            *i = self.im.apply(*i) * gain;
        }
        self.block += 1;
        if self.block.is_multiple_of(10) {
            std::mem::swap(&mut self.re, &mut self.warm_re);
            std::mem::swap(&mut self.im, &mut self.warm_im);
            self.warm_re.reset();
            self.warm_im.reset();
        }
    }
}
/// Symmetric look-ahead log-domain AGC, verbatim gain law and 155-sample attack/hold.
pub struct Agc {
    data: Vec<f32>,
    mag: Vec<f32>,
    shape: Vec<f32>,
    index: usize,
    beta: f32,
}
impl Agc {
    pub fn new() -> Self {
        let attack = 155;
        let hold = 155;
        let len = 2 * (attack + hold) + 1;
        let mut shape = vec![0.; len];
        for i in 0..attack {
            shape[i] = (0.5 - 0.5 * ((i + 1) as f32 * PI / (attack + 1) as f32).cos()).ln();
            shape[len - 1 - i] = shape[i];
        }
        let noise_in = 10_f32.powf(0.05 * 76.);
        let noise_out = 5000_f32;
        let beta = noise_in / (20000_f32 / (20000. - noise_out)).ln();
        Self {
            data: vec![0.; len],
            mag: vec![0.; len],
            shape,
            index: 0,
            beta,
        }
    }
    pub fn apply(&mut self, v: f32) -> f32 {
        let len = self.data.len();
        self.data[self.index] = v;
        self.mag[self.index] = (v + 1e-10).abs().ln();
        self.index = (self.index + 1) % len;
        let mut env = 1e-10_f32;
        let mut di = self.index;
        for w in &self.shape {
            env = env.max(self.mag[di] + w);
            di += 1;
            if di == len {
                di = 0;
            }
        }
        let amplitude = env.exp();
        let gain = 20000. * (1. - (-amplitude / self.beta).exp()) / amplitude;
        self.data[(self.index + len / 2) % len] * gain
    }
}
pub struct Modulator {
    sn: Vec<f32>,
    cs: Vec<f32>,
    index: usize,
    pitch: f32,
}
impl Modulator {
    pub fn new(pitch: f32) -> Self {
        let n = (RATE as f32 / pitch).round_ties_even() as usize;
        let step = (std::f64::consts::TAU / n as f64) as f32;
        let mut sn = vec![0.; n];
        let mut cs = vec![0.; n];
        sn[1] = (step as f64).sin() as f32;
        cs[0] = 1.;
        cs[1] = (step as f64).cos() as f32;
        for i in 2..n {
            cs[i] = cs[1] * cs[i - 1] - sn[1] * sn[i - 1];
            sn[i] = cs[1] * sn[i - 1] + sn[1] * cs[i - 1];
        }
        Self {
            sn,
            cs,
            index: 0,
            pitch,
        }
    }
    pub fn apply(&mut self, re: f32, im: f32, pitch: f32) -> f32 {
        if self.pitch != pitch {
            *self = Self::new(pitch);
        }
        let v = re * self.sn[self.index] - im * self.cs[self.index];
        self.index = (self.index + 1) % self.sn.len();
        v
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boxcar_impulse_matches_convolution() {
        let mut f = Average::new(3, 3);
        let out: Vec<_> = (0..10)
            .map(|i| f.apply(if i == 0 { 1. } else { 0. }))
            .collect();
        let expected = [1., 3., 6., 7., 6., 3., 1., 0., 0., 0.];
        for (a, b) in out.iter().zip(expected) {
            assert!((a - b / 27.).abs() < 1e-7);
        }
    }
    #[test]
    fn fading_is_smooth_and_nonnegative() {
        let mut rng = Random(12);
        let mut f = Fading::new(0.4, &mut rng);
        let mut v = vec![1.; 11025];
        f.apply(&mut v, &mut rng);
        assert!(v.iter().all(|v| v.is_finite() && *v >= 0.));
        assert!(v.windows(2).all(|p| (p[0] - p[1]).abs() < 0.03));
    }
    #[test]
    fn agc_limits_extreme_impulses() {
        let mut agc = Agc::new();
        let mut peak = 0_f32;
        for i in 0..2000 {
            peak = peak.max(agc.apply(if i == 700 { 1e7 } else { 0. }).abs());
        }
        assert!(peak > 19000. && peak <= 20001., "{peak}");
    }
    #[test]
    fn poisson_mean() {
        let mut r = Random(472);
        let mean = (0..10000).map(|_| r.poisson(2.) as f32).sum::<f32>() / 10000.;
        assert!((mean - 2.).abs() < 0.08);
    }
}

#[cfg(test)]
mod reference_tests {
    use super::*;
    fn compare(name: &str, actual: &[f32], bytes: &[u8], tolerance: f32) {
        let expected: Vec<_> = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        assert_eq!(actual.len(), expected.len());
        let error = actual
            .iter()
            .zip(expected)
            .map(|(a, b)| (a - b).abs())
            .fold(0_f32, f32::max);
        assert!(
            error < tolerance,
            "{name}: max sample error {error}, tolerance {tolerance}"
        );
    }
    #[test]
    fn quick_average_matches_pascal() {
        let mut f = Average::new(7, 3);
        let actual: Vec<_> = (0..1024)
            .map(|i| f.apply(((i as f64 * 0.173).sin() + (i as f64 * 0.071).cos()) as f32))
            .collect();
        compare(
            "QuickAvg",
            &actual,
            include_bytes!("../tests/fixtures/quick-average.f32"),
            0.0001,
        );
    }
    #[test]
    fn moving_average_matches_pascal() {
        let mut f = Average::new(15, 3);
        let actual: Vec<_> = (0..2048)
            .map(|i| f.apply(((i as f64 * 0.073).sin() + (i as f64 * 0.033).cos()) as f32))
            .collect();
        compare(
            "MovAvg",
            &actual,
            include_bytes!("../tests/fixtures/moving-average.f32"),
            0.001,
        );
    }
    #[test]
    fn agc_matches_pascal() {
        let mut f = Agc::new();
        let actual: Vec<_> = (0..4096)
            .map(|i| {
                let mut x = ((i as f64 * 0.37).sin() * 2000.) as f32;
                if (1000..1800).contains(&i) {
                    x *= 100.;
                }
                if i == 2800 {
                    x = 1e7;
                }
                f.apply(x)
            })
            .collect();
        compare(
            "AGC",
            &actual,
            include_bytes!("../tests/fixtures/agc.f32"),
            0.1,
        );
    }
    #[test]
    fn modulator_matches_pascal() {
        let mut f = Modulator::new(600.);
        let actual: Vec<_> = (0..4096)
            .map(|i| {
                f.apply(
                    (i as f64 * 0.013).sin() as f32,
                    (i as f64 * 0.023).cos() as f32,
                    600.,
                )
            })
            .collect();
        compare(
            "Modulator",
            &actual,
            include_bytes!("../tests/fixtures/modulator.f32"),
            0.0001,
        );
    }
}
#[cfg(test)]
mod fading_reference_test {
    use super::*;
    #[test]
    fn rayleigh_fading_matches_upstream_pascal() {
        let mut r = Random(144);
        let mut f = Fading::new(0.1, &mut r);
        f.set_bandwidth(0.4, &mut r);
        let mut actual = vec![1.; 110 * 512];
        f.apply(&mut actual, &mut r);
        let expected = include_bytes!("../tests/fixtures/fading.f32");
        let error = actual
            .iter()
            .zip(expected.as_chunks::<4>().0.iter())
            .map(|(a, b)| (a - f32::from_le_bytes(*b)).abs())
            .fold(0_f32, f32::max);
        assert!(error < 0.0001, "max QSB gain error {error}");
    }
}
