//! Each title gets one accent colour, pulled from its cover art when there is one.

use super::{Title, decode_media_path};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn css(self) -> String {
        format!("rgb({} {} {})", self.0, self.1, self.2)
    }
}

pub fn accent_for(title: &Title) -> Rgb {
    title
        .cover
        .as_deref()
        .and_then(|url| url.strip_prefix("http://joy.localhost/file/"))
        .and_then(decode_media_path)
        .and_then(|path| image::open(path).ok())
        .and_then(|img| dominant(&img.thumbnail(24, 36).to_rgb8()))
        .unwrap_or_else(|| from_name(&title.name))
}

/// Weighted average that favours vivid, mid-bright pixels, then tamed so it reads
/// well as a glow on dark glass.
fn dominant(img: &image::RgbImage) -> Option<Rgb> {
    let (mut r, mut g, mut b, mut total) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for p in img.pixels() {
        let [pr, pg, pb] = p.0.map(|c| c as f32 / 255.0);
        let max = pr.max(pg).max(pb);
        let min = pr.min(pg).min(pb);
        let sat = if max > 0.0 { (max - min) / max } else { 0.0 };
        let light = (max + min) / 2.0;
        let weight = sat * sat * (1.0 - (light - 0.5).abs() * 1.6).max(0.05);
        r += pr * weight;
        g += pg * weight;
        b += pb * weight;
        total += weight;
    }
    if total < 0.01 {
        return None;
    }
    let (h, s, _) = to_hsl(r / total, g / total, b / total);
    Some(from_hsl(h, s.clamp(0.35, 0.8), 0.6))
}

/// Stable colour for titles without art, so each tile still looks like itself.
fn from_name(name: &str) -> Rgb {
    let hash = name.bytes().fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619));
    // Skip the 250°–300° band: purple-blue gradients are the "default" look we want to avoid.
    let hue = (hash % 310) as f32;
    let hue = if hue >= 250.0 { hue + 50.0 } else { hue };
    from_hsl(hue, 0.55, 0.62)
}

fn to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < f32::EPSILON {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

fn from_hsl(h: f32, s: f32, l: f32) -> Rgb {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h / 60.0) as u32 % 6 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let to = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgb(to(r), to(g), to(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_colours_are_stable_and_avoid_purple() {
        for name in ["Portal 2", "Hades", "Celeste", "Spotify", "Discord", "Elden Ring"] {
            assert_eq!(from_name(name), from_name(name));
            let Rgb(r, g, b) = from_name(name);
            let (h, _, _) = to_hsl(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
            assert!(!(255.0..295.0).contains(&h), "{name} got hue {h}");
        }
    }

    #[test]
    fn picks_the_vivid_colour() {
        let mut img = image::RgbImage::from_pixel(4, 4, image::Rgb([20, 20, 20]));
        img.put_pixel(1, 1, image::Rgb([230, 60, 30]));
        img.put_pixel(2, 2, image::Rgb([220, 70, 40]));
        let Rgb(r, g, b) = dominant(&img).unwrap();
        assert!(r > g && r > b);
    }
}
