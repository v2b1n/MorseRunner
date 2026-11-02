// Copyright (C) 2004-2006 Alex Shovkoplyas, VE3NEA (referenced keying behavior).
// Copyright (C) 2026 Vladimir Vecgailis (Rust implementation).
// Rust implementation based on the timing and envelope behavior in
// VCL/MorseKey.pas; this source file is a Rust reimplementation.
// SPDX-License-Identifier: MPL-2.0
//! PARIS timing: one dot = 1.2 / WPM seconds; gaps 1/3/7 dots.
pub fn code(c: char) -> &'static str {
    match c {
        'A' => ".-",
        'B' => "-...",
        'C' => "-.-.",
        'D' => "-..",
        'E' => ".",
        'F' => "..-.",
        'G' => "--.",
        'H' => "....",
        'I' => "..",
        'J' => ".---",
        'K' => "-.-",
        'L' => ".-..",
        'M' => "--",
        'N' => "-.",
        'O' => "---",
        'P' => ".--.",
        'Q' => "--.-",
        'R' => ".-.",
        'S' => "...",
        'T' => "-",
        'U' => "..-",
        'V' => "...-",
        'W' => ".--",
        'X' => "-..-",
        'Y' => "-.--",
        'Z' => "--..",
        '0' => "-----",
        '1' => ".----",
        '2' => "..---",
        '3' => "...--",
        '4' => "....-",
        '5' => ".....",
        '6' => "-....",
        '7' => "--...",
        '8' => "---..",
        '9' => "----.",
        '/' => "-..-.",
        '?' => "..--..",
        '=' => "-...-",
        '.' => ".-.-.-",
        ',' => "--..--",
        '\\' => "...-.",
        _ => "",
    }
}
#[cfg(test)]
pub fn segments(text: &str) -> Vec<(bool, u32)> {
    let words: Vec<_> = text.split_whitespace().collect();
    let mut result = Vec::new();
    for (wi, word) in words.iter().enumerate() {
        let letters: Vec<_> = word
            .to_uppercase()
            .chars()
            .filter(|c| !code(*c).is_empty())
            .collect();
        for (ci, c) in letters.iter().enumerate() {
            for (i, d) in code(*c).chars().enumerate() {
                if i > 0 {
                    result.push((false, 1));
                }
                result.push((true, if d == '.' { 1 } else { 3 }));
            }
            if ci + 1 < letters.len() {
                result.push((false, 3));
            }
        }
        if wi + 1 < words.len() {
            result.push((false, 7));
        }
    }
    result
}
// VCL/MorseKey.pas reference: integrated Blackman–Harris ramps,
// exact trailing ~ marker, original dot rounding and block padding.
pub fn encode(text: &str) -> String {
    let mut encoded = String::new();
    for c in text.to_uppercase().chars() {
        if c == ' ' || c == '_' {
            encoded.push(' ');
        } else if !code(c).is_empty() {
            encoded.push_str(code(c));
            encoded.push(' ');
        }
    }
    if !encoded.is_empty() {
        encoded.pop();
        encoded.push('~');
    }
    encoded
}
pub fn envelope(text: &str, wpm: f32, rate: u32) -> Vec<f32> {
    let encoded = encode(text);
    if encoded.is_empty() {
        return Vec::new();
    }
    let dot = (0.1 * rate as f32 * 12. / wpm.clamp(5., 120.)).round_ties_even() as usize;
    let ramp_len = ((2.7 * 0.005 * rate as f32).round_ties_even() as usize).min(dot);
    let mut ramp = Vec::with_capacity(ramp_len);
    let mut sum = 0_f32;
    for i in 0..ramp_len {
        let x = i as f64 / ramp_len as f64;
        sum += (0.35875 - 0.48829 * (2. * std::f64::consts::PI * x).cos()
            + 0.14128 * (4. * std::f64::consts::PI * x).cos()
            - 0.01168 * (6. * std::f64::consts::PI * x).cos()) as f32;
        ramp.push(sum);
    }
    let scale = 1. / sum;
    for v in &mut ramp {
        *v *= scale;
    }
    let units: usize = encoded
        .chars()
        .map(|c| match c {
            '.' => 2,
            '-' => 4,
            ' ' => 2,
            '~' => 1,
            _ => 0,
        })
        .sum();
    let block = crate::dsp::BLOCK;
    let mut out = vec![0.; (units * dot + ramp_len).div_ceil(block) * block];
    let mut p = 0;
    for c in encoded.chars() {
        match c {
            '.' | '-' => {
                let duration = if c == '.' { 1 } else { 3 };
                out[p..p + ramp_len].copy_from_slice(&ramp);
                p += ramp_len;
                let n = duration * dot - ramp_len;
                out[p..p + n].fill(1.);
                p += n;
                for v in ramp.iter().rev() {
                    out[p] = *v;
                    p += 1;
                }
                p += dot - ramp_len;
            }
            ' ' => p += 2 * dot - ramp_len,
            '~' => p += dot - ramp_len,
            _ => {}
        }
    }
    out
}
pub fn prefix(call: &str) -> String {
    let upper = call.to_uppercase();
    let parts: Vec<_> = upper
        .split('/')
        .filter(|p| !["P", "M", "MM", "QRP"].contains(p))
        .collect();
    let s = parts.iter().min_by_key(|p| p.len()).copied().unwrap_or("");
    if let Some(i) = s.rfind(|c: char| c.is_ascii_digit()) {
        s[..=i].to_owned()
    } else {
        format!("{}0", s.chars().take(2).collect::<String>())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paris_is_50_units_with_word_gap() {
        let units: u32 = segments("PARIS").iter().map(|(_, n)| n).sum();
        assert_eq!(units + 7, 50);
    }
    #[test]
    fn word_spacing() {
        assert_eq!(segments("E E"), vec![(true, 1), (false, 7), (true, 1)]);
    }
    #[test]
    fn original_envelope_shape_and_padding() {
        let e = envelope("E", 20., 11025);
        assert_eq!(e.len(), 2560); // 3*662 + 149, rounded to a 512-sample block
        assert!(e[0] < 0.00001);
        assert_eq!(e[300], 1.);
        assert_eq!(*e.last().unwrap(), 0.);
    }
    #[test]
    fn contest_prefixes() {
        assert_eq!(prefix("DL1ABC/P"), "DL1");
        assert_eq!(prefix("DL1ABC"), "DL1");
        assert_eq!(prefix("F/G4ABC"), "F0");
    }
}

#[cfg(test)]
mod reference_tests {
    use super::*;
    #[test]
    fn keyer_matches_upstream_pascal_samples() {
        for (name, text, wpm, bytes) in [
            (
                "E",
                "E",
                20.,
                include_bytes!("../tests/fixtures/key-e.f32").as_slice(),
            ),
            (
                "CQ",
                "CQ DL1ABC TEST",
                30.,
                include_bytes!("../tests/fixtures/key-cq.f32").as_slice(),
            ),
            (
                "correction",
                "5NN TT1 EEEEE 021",
                80.,
                include_bytes!("../tests/fixtures/key-error.f32").as_slice(),
            ),
        ] {
            let actual = envelope(text, wpm, crate::dsp::RATE);
            let expected: Vec<_> = bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b))
                .collect();
            assert_eq!(actual.len(), expected.len(), "{name}");
            let error = actual
                .iter()
                .zip(expected)
                .map(|(a, b)| (a - b).abs())
                .fold(0_f32, f32::max);
            assert!(error < 0.00001, "{name}: max sample error {error}");
        }
    }
}
