//! Embedded sprite sheets (PNG + Aseprite "Array" JSON) decoded once at startup.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

include!(concat!(env!("OUT_DIR"), "/sprites_index.rs"));

/// One frame: row-major pixels, `None` = transparent.
pub struct Frame {
    pub w: u16,
    pub h: u16,
    pub px: Vec<Option<[u8; 3]>>,
    pub ms: u32,
}

impl Frame {
    pub fn get(&self, x: u16, y: u16) -> Option<[u8; 3]> {
        self.px[(y * self.w + x) as usize]
    }
}

pub struct Sheet {
    pub frames: Vec<Frame>,
    tags: HashMap<String, (usize, usize)>,
}

impl Sheet {
    /// The frame of `tag` shown `elapsed_ms` into a looping playback. Unknown tag → frame 0.
    pub fn frame(&self, tag: &str, elapsed_ms: u64) -> &Frame {
        let Some(&(from, to)) = self.tags.get(tag) else {
            return &self.frames[0];
        };
        let frames = &self.frames[from..=to];
        let total: u64 = frames.iter().map(|f| f.ms.max(1) as u64).sum();
        let mut t = elapsed_ms % total;
        for f in frames {
            if t < f.ms.max(1) as u64 {
                return f;
            }
            t -= f.ms.max(1) as u64;
        }
        &frames[0]
    }
}

#[derive(Deserialize)]
struct Json {
    frames: Vec<JsonFrame>,
    meta: Meta,
}
#[derive(Deserialize)]
struct JsonFrame {
    frame: Rect,
    duration: u32,
}
#[derive(Deserialize)]
struct Rect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}
#[derive(Deserialize)]
struct Meta {
    #[serde(rename = "frameTags", default)]
    frame_tags: Vec<JsonTag>,
}
#[derive(Deserialize)]
struct JsonTag {
    name: String,
    from: usize,
    to: usize,
}

fn decode(png: &[u8], json: &str) -> Sheet {
    let img = image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .expect("embedded png")
        .to_rgba8();
    let j: Json = serde_json::from_str(json).expect("embedded sprite json");
    let frames = j
        .frames
        .iter()
        .map(|f| {
            let r = &f.frame;
            let mut px = Vec::with_capacity((r.w * r.h) as usize);
            for y in r.y..r.y + r.h {
                for x in r.x..r.x + r.w {
                    let p = img.get_pixel(x, y).0;
                    px.push((p[3] >= 128).then_some([p[0], p[1], p[2]]));
                }
            }
            Frame {
                w: r.w as u16,
                h: r.h as u16,
                px,
                ms: f.duration,
            }
        })
        .collect();
    let tags = j
        .meta
        .frame_tags
        .into_iter()
        .map(|t| (t.name, (t.from, t.to)))
        .collect();
    Sheet { frames, tags }
}

/// All sheets by name, decoded on first use.
pub fn sheets() -> &'static HashMap<&'static str, Sheet> {
    static SHEETS_CACHE: OnceLock<HashMap<&'static str, Sheet>> = OnceLock::new();
    SHEETS_CACHE.get_or_init(|| {
        SHEETS
            .iter()
            .map(|(name, png, json)| (*name, decode(png, json)))
            .collect()
    })
}

/// `f` at twice the size (each pixel 2×2), for props that must read at a glance.
pub fn scale2(f: &Frame) -> Frame {
    let (w, h) = (f.w * 2, f.h * 2);
    let px = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| f.px[((y / 2) * f.w + x / 2) as usize])
        .collect();
    Frame { w, h, px, ms: f.ms }
}

/// A copy of `f` with palette colors swapped (hero skins, M14).
pub fn recolor(f: &Frame, swaps: &[(char, char)]) -> Frame {
    let map: Vec<([u8; 3], [u8; 3])> = swaps
        .iter()
        .map(|&(a, b)| (crate::palette::rgb(a), crate::palette::rgb(b)))
        .collect();
    let swap = |c: [u8; 3]| map.iter().find(|(a, _)| *a == c).map_or(c, |(_, b)| *b);
    Frame {
        w: f.w,
        h: f.h,
        px: f.px.iter().map(|p| p.map(swap)).collect(),
        ms: f.ms,
    }
}

pub fn sheet(name: &str) -> &'static Sheet {
    sheets()
        .get(name)
        .unwrap_or_else(|| panic!("missing sprite sheet `{name}`"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m1_sheets_load() {
        for class in [
            "knight",
            "mage",
            "rogue",
            "ranger",
            "cleric",
            "bard",
            "sahur_knight",
            "wild_card",
        ] {
            let s = sheet(&format!("hero_{class}"));
            assert_eq!((s.frames[0].w, s.frames[0].h), (16, 16));
            assert!(s.tags.contains_key("walk"));
            assert_eq!(sheet(&format!("face_{class}")).frames[0].w, 8);
        }
        assert_eq!(sheet("slime").frames[0].w, 10);
        for id in [
            "test_golem",
            "flaky_ghost",
            "snapshot_mimic",
            "build_wyrm",
            "linker_kraken",
            "dependency_hydra",
            "lint_imp",
            "typo_gremlin",
            "slime_king",
            "sahur_knocker",
        ] {
            for (tier, h) in [("little", 12), ("normal", 20), ("big", 28)] {
                let s = sheet(&format!("boss_{id}_{tier}"));
                assert_eq!(s.frames[0].h, h);
                assert!(
                    ["idle", "laugh", "die"]
                        .iter()
                        .all(|t| s.tags.contains_key(*t))
                );
            }
        }
        assert_eq!(sheet("rubber_duck").frames[0].w, 8);
        assert_eq!(sheet("campfire").frames.len(), 3);
    }

    #[test]
    fn frame_loops_by_duration() {
        let s = sheet("slime");
        let a = s.frame("idle", 0) as *const Frame;
        let b = s.frame("idle", 300) as *const Frame;
        assert_ne!(a, b);
        assert_eq!(a, s.frame("idle", 600) as *const Frame);
    }
}
