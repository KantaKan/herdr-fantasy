//! Asset pipeline (ART.md §8): `assets/placeholders/*.txt` → PNG sheet + Aseprite-style
//! "Array" JSON in OUT_DIR, embedded by `src/render/sprites.rs`. A real export in
//! `assets/sprites/<name>.png` + `.json` replaces the placeholder of the same name.

#[allow(dead_code)]
#[path = "src/palette.rs"]
mod palette;

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// Hero/face templates use `M` main, `D` shadow, `A` accent (ART.md §4 placeholder rule).
const CLASSES: [(&str, char, char, char); 6] = [
    ("knight", 's', 'S', 'r'),
    ("mage", 'b', 'n', 'y'),
    ("rogue", 'd', 'p', 'C'),
    ("ranger", 'g', 't', 'G'),
    ("cleric", 'w', 's', 'y'),
    ("bard", 'o', 'r', 'y'),
];

struct Frame {
    rows: Vec<String>,
    ms: u32,
}

struct Tag {
    name: String,
    from: usize,
    to: usize,
}

fn parse(name: &str, text: &str) -> (Vec<Frame>, Vec<Tag>) {
    let mut frames: Vec<Frame> = Vec::new();
    let mut tags: Vec<Tag> = Vec::new();
    let mut cur: Vec<String> = Vec::new();
    let mut ms = 100;
    let flush = |cur: &mut Vec<String>, frames: &mut Vec<Frame>, tags: &mut Vec<Tag>, ms: u32| {
        if cur.is_empty() {
            return;
        }
        frames.push(Frame {
            rows: std::mem::take(cur),
            ms,
        });
        tags.last_mut()
            .unwrap_or_else(|| panic!("{name}: frame before first `# tag`"))
            .to = frames.len() - 1;
    };
    for line in text.lines().map(str::trim_end) {
        if let Some(rest) = line.strip_prefix("# tag ") {
            flush(&mut cur, &mut frames, &mut tags, ms);
            let mut it = rest.split_whitespace();
            let tag = it.next().expect("tag name").to_string();
            ms = it.next().and_then(|v| v.parse().ok()).unwrap_or(100);
            tags.push(Tag {
                name: tag,
                from: frames.len(),
                to: frames.len(),
            });
        } else if line.is_empty() {
            flush(&mut cur, &mut frames, &mut tags, ms);
        } else {
            cur.push(line.to_string());
        }
    }
    flush(&mut cur, &mut frames, &mut tags, ms);
    let (w, h) = (frames[0].rows[0].len(), frames[0].rows.len());
    for (i, f) in frames.iter().enumerate() {
        assert!(
            f.rows.len() == h && f.rows.iter().all(|r| r.len() == w),
            "{name}: frame {i} is not {w}x{h}"
        );
    }
    (frames, tags)
}

fn emit(out: &Path, name: &str, frames: &[Frame], tags: &[Tag], subst: &dyn Fn(char) -> char) {
    let (w, h) = (frames[0].rows[0].len(), frames[0].rows.len());
    let mut img = image::RgbaImage::new((w * frames.len()) as u32, h as u32);
    for (i, f) in frames.iter().enumerate() {
        for (y, row) in f.rows.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                if c == '.' {
                    continue;
                }
                let key = subst(c);
                assert!(
                    palette::PALETTE.iter().any(|(k, _)| *k == key),
                    "{name}: `{c}` is not a palette key"
                );
                let [r, g, b] = palette::rgb(key);
                img.put_pixel((i * w + x) as u32, y as u32, image::Rgba([r, g, b, 255]));
            }
        }
    }
    img.save(out.join(format!("{name}.png"))).unwrap();

    let mut json = String::from("{\"frames\":[");
    for (i, f) in frames.iter().enumerate() {
        let sep = if i > 0 { "," } else { "" };
        let x = i * w;
        write!(json, "{sep}{{\"filename\":\"{name} {i}\",\"frame\":{{\"x\":{x},\"y\":0,\"w\":{w},\"h\":{h}}},\"duration\":{}}}", f.ms).unwrap();
    }
    write!(
        json,
        "],\"meta\":{{\"image\":\"{name}.png\",\"size\":{{\"w\":{},\"h\":{h}}},\"frameTags\":[",
        w * frames.len()
    )
    .unwrap();
    for (i, t) in tags.iter().enumerate() {
        let sep = if i > 0 { "," } else { "" };
        write!(
            json,
            "{sep}{{\"name\":\"{}\",\"from\":{},\"to\":{},\"direction\":\"forward\"}}",
            t.name, t.from, t.to
        )
        .unwrap();
    }
    json.push_str("]}}");
    fs::write(out.join(format!("{name}.json")), json).unwrap();
}

fn main() {
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=src/palette.rs");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let mut names = Vec::new();

    let mut entries: Vec<_> = fs::read_dir("assets/placeholders")
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .collect();
    entries.sort();
    for path in entries {
        let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
        let (frames, tags) = parse(&stem, &fs::read_to_string(&path).unwrap());
        if stem == "hero" || stem == "face" || stem == "portrait" {
            for (class, m, d, a) in CLASSES {
                let name = format!("{stem}_{class}");
                let subst = move |c| match c {
                    'M' => m,
                    'D' => d,
                    'A' => a,
                    c => c,
                };
                emit(&out, &name, &frames, &tags, &subst);
                names.push(name);
            }
        } else {
            emit(&out, &stem, &frames, &tags, &|c| c);
            names.push(stem);
        }
    }

    // Real art wins over placeholders.
    if let Ok(dir) = fs::read_dir("assets/sprites") {
        for p in dir.map(|e| e.unwrap().path()) {
            if p.extension().is_some_and(|e| e == "png") && p.with_extension("json").exists() {
                let stem = p.file_stem().unwrap().to_str().unwrap().to_string();
                fs::copy(&p, out.join(format!("{stem}.png"))).unwrap();
                fs::copy(p.with_extension("json"), out.join(format!("{stem}.json"))).unwrap();
                if !names.contains(&stem) {
                    names.push(stem);
                }
            }
        }
    }

    let mut index = String::from("pub const SHEETS: &[(&str, &[u8], &str)] = &[\n");
    for n in &names {
        writeln!(
            index,
            "    (\"{n}\", include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{n}.png\")), include_str!(concat!(env!(\"OUT_DIR\"), \"/{n}.json\"))),"
        )
        .unwrap();
    }
    index.push_str("];\n");
    fs::write(out.join("sprites_index.rs"), index).unwrap();
}
