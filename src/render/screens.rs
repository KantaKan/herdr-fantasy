//! Quest log, character sheet, town and help screens (SPEC §5.6–5.8).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use super::halfblock::{self, rgb};
use super::sprites::sheet;
use crate::game::hero::Hero;
use crate::game::town;
use crate::game::world::World;
use crate::game::xp::xp_to_next;
use crate::palette;

fn c(key: char) -> Color {
    rgb(palette::rgb(key))
}

/// Write `s` at (x, y), clipped to `area`; returns the column after it.
fn put(buf: &mut Buffer, area: Rect, x: u16, y: u16, s: &str, fg: char, bold: bool) -> u16 {
    if y >= area.bottom() || x >= area.right() {
        return x;
    }
    let mut style = Style::default().fg(c(fg)).bg(c('d'));
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    buf.set_stringn(x, y, s, (area.right() - x) as usize, style)
        .0
}

fn mmss(ms: u64) -> String {
    let s = ms / 1000;
    format!("{:02}:{:02}", s / 60, s % 60)
}

/// Quest log for the selected hero's repo, newest first, with chapter headings.
pub fn quests(w: &World, hero: Option<&Hero>, scroll: usize, area: Rect, buf: &mut Buffer) {
    let repo = hero.map(World::repo_of);
    let rows: Vec<_> = w
        .log
        .iter()
        .rev()
        .filter(|e| repo.is_none() || e.repo.is_none() || e.repo == repo)
        .collect();
    let title = format!(
        " Quest log · {} quests  (↑/↓ scroll · Esc back)",
        rows.len()
    );
    put(buf, area, area.x, area.y, &title, 'y', true);
    if rows.is_empty() {
        put(
            buf,
            area,
            area.x + 1,
            area.y + 2,
            "No quests finished yet — go adventure!",
            's',
            false,
        );
        return;
    }
    let mut y = area.y + 2;
    let mut chapter = None;
    let title_w = area.width.saturating_sub(40).max(10) as usize;
    for e in rows.into_iter().skip(scroll) {
        if y >= area.bottom() {
            break;
        }
        if chapter != Some((e.hero.clone(), e.chapter)) {
            chapter = Some((e.hero.clone(), e.chapter));
            put(
                buf,
                area,
                area.x + 1,
                y,
                &format!("── {} · Chapter {} ──", e.hero, e.chapter),
                'S',
                false,
            );
            y += 1;
            if y >= area.bottom() {
                break;
            }
        }
        let (mark, col) = if e.result == "complete" {
            ("✓", 'g')
        } else {
            ("✗", 'r')
        };
        let mut x = put(buf, area, area.x + 1, y, mark, col, true);
        let title: String = e.title.chars().take(title_w).collect();
        x = put(
            buf,
            area,
            x + 1,
            y,
            &format!("{title:<title_w$}"),
            'w',
            false,
        );
        x = put(
            buf,
            area,
            x + 1,
            y,
            &format!("{:>6}", mmss(e.duration_ms)),
            's',
            false,
        );
        x = put(
            buf,
            area,
            x + 1,
            y,
            &format!("{:>6}", format!("+{}", e.xp)),
            'y',
            false,
        );
        let bosses: String = e.bosses.chars().take(8).collect();
        put(
            buf,
            area,
            x + 2,
            y,
            &bosses,
            if e.bosses.contains('✗') { 'o' } else { 'g' },
            false,
        );
        y += 1;
    }
}

/// Character sheet of one hero (SPEC §5.8).
pub fn character(w: &World, hero: Option<&Hero>, area: Rect, buf: &mut Buffer) {
    let Some(h) = hero else {
        put(
            buf,
            area,
            area.x + 1,
            area.y + 1,
            "No hero selected.",
            's',
            false,
        );
        return;
    };
    let portrait = area.width >= 64 && area.height >= 12;
    let tx = if portrait {
        let f = sheet(&format!("portrait_{}", h.class.id())).frame("idle", w.clock_ms);
        let frame = Rect::new(area.x + 1, area.y, 32, area.height);
        halfblock::draw(buf, frame, f, 0, 0, false);
        area.x + 35
    } else {
        area.x + 1
    };
    let mut y = area.y;
    let x = put(
        buf,
        area,
        tx,
        y,
        &format!(" {} ", h.name),
        h.class.color(),
        true,
    );
    put(
        buf,
        area,
        x + 1,
        y,
        &format!("{} · Lv {}", h.class.name(), h.level),
        'w',
        false,
    );
    y += 1;
    let need = xp_to_next(h.level);
    let filled = (h.xp * 20 / need.max(1)) as usize;
    let x = put(buf, area, tx, y, "XP  ", 's', false);
    let x = put(buf, area, x, y, &"▓".repeat(filled.min(20)), 'y', false);
    let x = put(
        buf,
        area,
        x,
        y,
        &"░".repeat(20 - filled.min(20)),
        'S',
        false,
    );
    put(
        buf,
        area,
        x + 1,
        y,
        &format!("{} / {need}", h.xp),
        's',
        false,
    );
    y += 2;
    let stats = h.life.stats();
    let top = stats.iter().copied().max().unwrap_or(0).max(10);
    for (name, v) in ["STR", "INT", "DEX", "VIT", "LUK"].iter().zip(stats) {
        let cells = (v as usize * 20) / top as usize;
        let x = put(buf, area, tx, y, &format!("{name} "), 'w', true);
        let x = put(buf, area, x, y, &"█".repeat(cells), 'c', false);
        put(buf, area, x + 1, y, &v.to_string(), 'w', false);
        y += 1;
    }
    y += 1;
    let l = &h.life;
    for line in [
        format!(
            "Quests {} · Chapters {} · Rests {}",
            l.quests,
            l.chapters + 1,
            l.rests
        ),
        format!(
            "Bosses {} won / {} lost · best streak {}",
            l.bosses_won, l.bosses_lost, l.best_streak
        ),
        format!("Shadows raised {} · Gold {} G", l.shadows, l.gold),
        match (
            &h.pet,
            crate::game::skins::for_level(h.level).filter(|_| w.skins),
        ) {
            (Some(p), Some(s)) => format!("Pet {p} · Skin {}", s.name),
            (Some(p), None) => format!("Pet {p} · next skin at Lv 5"),
            (None, Some(s)) => format!("Skin {}", s.name),
            (None, None) => "Next skin at Lv 5".into(),
        },
    ] {
        put(buf, area, tx, y, &line, 's', false);
        y += 1;
    }
    use crate::game::badges::{BADGES, by_id};
    if l.daily.day > 0 {
        y += 1;
        let (name, n, target) = crate::game::daily::progress(l);
        let x = put(buf, area, tx, y, "Daily", 'w', true);
        let x = put(buf, area, x + 1, y, name, 's', false);
        let (s, c) = if l.daily.done {
            ("✓ done".to_string(), 'G')
        } else {
            (format!("{n}/{target}"), 'y')
        };
        put(buf, area, x + 1, y, &s, c, false);
        y += 1;
    }
    y += 1;
    let x = put(buf, area, tx, y, "Badges", 'w', true);
    put(
        buf,
        area,
        x + 1,
        y,
        &format!("{}/{}", l.badges.len(), BADGES.len()),
        's',
        false,
    );
    y += 1;
    let mut x = tx;
    for b in l.badges.iter().filter_map(|id| by_id(id)) {
        if x > tx && x + b.name.len() as u16 + 3 > area.right() {
            x = tx;
            y += 1;
        }
        x = put(buf, area, x, y, &format!("🏆{} ", b.name), 'y', false) + 1;
    }
    if !l.badges.is_empty() {
        y += 1;
    }
    if let Some(next) = BADGES
        .iter()
        .find(|b| !l.badges.iter().any(|id| id == b.id))
    {
        put(
            buf,
            area,
            tx,
            y,
            &format!("Next: {} — {}", next.name, next.how),
            'S',
            false,
        );
        y += 1;
    }
    let gear = crate::game::loot::equipped(&l.loot);
    if !gear.is_empty() {
        y += 1;
        put(buf, area, tx, y, "Gear", 'w', true);
        y += 1;
    }
    for item in gear {
        let stat = crate::game::loot::STAT_NAMES[item.stat];
        let x = put(buf, area, tx, y, &item.name, item.rarity.color(), false);
        put(
            buf,
            area,
            x + 1,
            y,
            &format!("+{} {stat}", item.bonus),
            'G',
            false,
        );
        y += 1;
    }
}

/// Weekly recap (`w`, M15): the last 7 days of the quest log.
pub fn week(w: &World, area: Rect, buf: &mut Buffer) {
    use crate::game::recap::{self, WEEKDAYS};
    let today = if w.today > 0 {
        w.today
    } else {
        w.log.iter().map(|e| e.day).max().unwrap_or(0)
    };
    let r = recap::week(&w.log, today);
    let x0 = area.x + 1;
    let mut y = area.y;
    put(buf, area, x0, y, " This week", 'y', true);
    y += 2;
    let repo_name = |key: &str| {
        w.heroes
            .iter()
            .filter_map(|h| h.seat.as_ref())
            .find(|s| s.repo.key == key)
            .map_or_else(|| w.repo.clone(), |s| s.repo.name.clone())
    };
    let lines = [
        format!("Quests {} · XP {}", r.quests, r.xp),
        format!(
            "Bosses {} won / {} lost · best run {}",
            r.won, r.lost, r.best_run
        ),
        match &r.top_hero {
            Some((h, xp)) => format!("MVP {h} ({xp} XP)"),
            None => "MVP —".into(),
        },
        match &r.top_repo {
            Some((key, n)) => format!("Busiest repo {} ({n} quests)", repo_name(key)),
            None => "Busiest repo —".into(),
        },
    ];
    for line in lines {
        put(buf, area, x0, y, &line, 's', false);
        y += 1;
    }
    y += 1;
    let top = r.per_day.iter().copied().max().unwrap_or(0).max(1);
    for (i, n) in r.per_day.iter().enumerate() {
        let day = today.saturating_sub(6) + i as u32;
        let x = put(buf, area, x0, y, WEEKDAYS[recap::weekday(day)], 'w', i == 6);
        let bar = (*n as usize * 24).div_ceil(top as usize);
        let x = put(buf, area, x + 1, y, &"█".repeat(bar), 'c', false);
        put(buf, area, x + 1, y, &n.to_string(), 's', false);
        y += 1;
    }
}

/// Settings (`s`, M18): ↑/↓ pick a row, ←/→ change it; saved to config.toml.
pub fn settings(w: &World, hero: usize, row: usize, area: Rect, buf: &mut Buffer) {
    use crate::game::settings::{ROWS, show};
    let x0 = area.x + 1;
    let name = w.heroes.get(hero).map_or("—", |h| h.name.as_str());
    let x = put(buf, area, x0, area.y, " Settings", 'y', true);
    put(
        buf,
        area,
        x + 2,
        area.y,
        &format!("for {name}  (Tab: next hero)"),
        's',
        false,
    );
    for (i, r) in ROWS.iter().enumerate() {
        let y = area.y + 2 + i as u16;
        let (label, value) = show(w, hero, *r);
        let here = i == row;
        put(buf, area, x0, y, if here { "▶" } else { " " }, 'y', true);
        put(
            buf,
            area,
            x0 + 2,
            y,
            &label,
            if here { 'w' } else { 's' },
            here,
        );
        put(
            buf,
            area,
            x0 + 26,
            y,
            &format!("◀ {value} ▶"),
            if here { 'y' } else { 'c' },
            here,
        );
    }
    let y = area.y + 3 + ROWS.len() as u16;
    put(
        buf,
        area,
        x0,
        y,
        "↑/↓ choose · ←/→ change · saved to config.toml · Esc back",
        'S',
        false,
    );
}

/// First-run welcome card in the middle of the scene; any key closes it.
pub fn welcome(scene: Rect, buf: &mut Buffer) {
    let lines = [
        ("Welcome to herdr fantasy!", 'y'),
        ("Your coding agents are the party.", 'w'),
        ("They level up as they work.", 'w'),
        ("", 'w'),
        ("c hero · w week · t town · q quests", 'c'),
        ("s settings: class, pet · ? help", 'c'),
        ("", 'w'),
        ("press any key", 'S'),
    ];
    let w = 40.min(scene.width);
    let h = (lines.len() as u16 + 2).min(scene.height);
    let x = scene.x + (scene.width - w) / 2;
    let y = scene.y + (scene.height - h) / 2;
    let card = Rect::new(x, y, w, h);
    for cy in card.top()..card.bottom() {
        for cx in card.left()..card.right() {
            buf[(cx, cy)].reset();
            buf[(cx, cy)].set_bg(crate::render::halfblock::rgb(crate::palette::rgb('k')));
        }
    }
    for (i, (s, c)) in lines.iter().enumerate() {
        let len = s.chars().count() as u16;
        put(
            buf,
            card,
            x + (w.saturating_sub(len)) / 2,
            y + 1 + i as u16,
            s,
            *c,
            i == 0,
        );
    }
}

/// Keys (`?`).
pub fn help(area: Rect, buf: &mut Buffer) {
    let lines = [
        ("t", "town — your repo as a village"),
        ("q", "quest log"),
        ("c", "character sheet of the selected hero"),
        ("w", "this week's recap"),
        ("s", "settings: class, pet, skins, memes, motion"),
        ("1-4", "in town: buy from the shop, or rest at the inn"),
        ("← →", "select hero"),
        ("Enter", "jump to that agent's pane"),
        ("Esc", "back to battle"),
        ("Ctrl-C", "quit the stage"),
    ];
    put(buf, area, area.x, area.y, " Keys", 'y', true);
    for (i, (k, what)) in lines.iter().enumerate() {
        let y = area.y + 2 + i as u16;
        let x = put(buf, area, area.x + 2, y, &format!("{k:<7}"), 'w', true);
        put(buf, area, x + 1, y, what, 's', false);
    }
}

/// The town of the selected hero's repo: buildings, decorations, heroes by their last
/// file. `full` draws sprites; otherwise a one-line summary.
/// The town's shop and inn (M26) across the top of the town scene: wares 1-3, the
/// inn 4, and the selected hero's gold.
fn shop_window(w: &World, hero: Option<&Hero>, scene: Rect, buf: &mut Buffer) {
    use crate::game::loot::STAT_NAMES;
    use crate::game::shop::{INN_PRICE, stock};
    if scene.height < 6 || scene.width < 40 {
        return;
    }
    let r = Rect::new(scene.x, scene.y, scene.width, 4); // in the sky: labels sit at the bottom
    super::layout::window(buf, r, 'w');
    let wares: Vec<String> = stock(&w.theme)
        .iter()
        .enumerate()
        .map(|(i, s)| {
            format!(
                "{} {} +{} {} {}G",
                i + 1,
                s.name,
                s.bonus,
                STAT_NAMES[s.stat],
                s.price
            )
        })
        .collect();
    let (x, end) = (r.x + 2, r.right() - 1);
    let sx = put(buf, r, x, r.y + 1, "SHOP ", 'y', true);
    put(buf, r, sx, r.y + 1, &wares.join("   "), 'w', false);
    let ix = put(buf, r, x, r.y + 2, "INN  ", 'y', true);
    let ix = put(
        buf,
        r,
        ix,
        r.y + 2,
        &format!("4 Rest, Well Rested {INN_PRICE}G"),
        'w',
        false,
    );
    if let Some(h) = hero {
        let s = format!("{}: {} G  (1-4 buy)", h.name, h.life.gold);
        let gx = end.saturating_sub(s.chars().count() as u16 + 1).max(ix + 2);
        put(buf, r, gx, r.y + 2, &s, 'c', false);
    }
}

pub fn town_scene(w: &World, hero: Option<&Hero>, scene: Rect, buf: &mut Buffer, full: bool) {
    town_buildings(w, hero, scene, buf, full);
    shop_window(w, hero, scene, buf);
}

fn town_buildings(w: &World, hero: Option<&Hero>, scene: Rect, buf: &mut Buffer, full: bool) {
    let key = hero.map_or(w.repo_key.clone(), World::repo_of);
    let Some(t) = w.towns.get(&key) else {
        put(
            buf,
            scene,
            scene.x + 1,
            scene.y,
            "Surveying the town…",
            's',
            false,
        );
        return;
    };
    let ground = (scene.height as i32 - 1) * 2;
    let label_y = scene.bottom() - 1;
    if !full {
        let mut x = put(buf, scene, scene.x + 1, scene.y, t.theme.name(), 'y', true);
        x = put(
            buf,
            scene,
            x + 1,
            scene.y,
            &format!("· {} commits", t.commits),
            's',
            false,
        );
        let _ = x;
        let mut x = scene.x + 1;
        for (i, b) in t.buildings.iter().enumerate() {
            let here: String = w
                .heroes
                .iter()
                .filter(|h| {
                    h.last_file
                        .as_deref()
                        .and_then(|f| town::building_for(f, t))
                        == Some(i)
                })
                .filter_map(|h| h.name.chars().next())
                .collect();
            let label = if here.is_empty() {
                format!("[{}]", b.name)
            } else {
                format!("[{} {here}]", b.name)
            };
            x = put(
                buf,
                scene,
                x,
                scene.y + 1,
                &label,
                if here.is_empty() { 'w' } else { 'y' },
                false,
            ) + 1;
        }
        return;
    }
    // Decorations stand in the background, spread across the whole town; the
    // buildings go in front of them.
    let n = t.decorations.len() as i32;
    for (k, d) in t.decorations.iter().enumerate() {
        let f = sheet(&format!("deco_{d}")).frame("a", w.clock_ms);
        let dx = (scene.width as i32 - f.w as i32) * (2 * k as i32 + 1) / (2 * n.max(1));
        halfblock::draw(buf, scene, f, dx, ground - f.h as i32 - 2, false);
    }
    let mut x: i32 = 1;
    let mut spots = Vec::new();
    for b in &t.buildings {
        let f = sheet(&format!("town_{}_{}", t.theme.id(), b.tier)).frame("a", w.clock_ms);
        if x + f.w as i32 > scene.width as i32 {
            break;
        }
        halfblock::draw(buf, scene, f, x, ground - f.h as i32, false);
        let name: String = b.name.chars().take(f.w as usize + 2).collect();
        let lx = (scene.x as i32 + x + (f.w as i32 - name.chars().count() as i32) / 2)
            .max(scene.x as i32);
        let style = Style::default().fg(c('y')).bg(c('t'));
        buf.set_stringn(
            lx as u16,
            label_y,
            &name,
            scene.right().saturating_sub(lx as u16) as usize,
            style,
        );
        spots.push((x, f.w as i32));
        x += f.w as i32 + 3;
    }
    // Heroes stand beside the building of the file they last touched; others wander
    // the town square on the left.
    let mut crowd = vec![0; spots.len()];
    for (n, h) in w.heroes.iter().enumerate() {
        let face = sheet(&format!("face_{}", h.class.id())).frame("idle", h.anim_ms);
        let at = h
            .last_file
            .as_deref()
            .and_then(|f| town::building_for(f, t));
        let hx = match at.filter(|&i| i < spots.len()) {
            Some(i) => {
                crowd[i] += 1;
                spots[i].0 + spots[i].1 - 3 + (crowd[i] - 1) * 5
            }
            None => n as i32 * 9 - 2,
        };
        halfblock::draw(buf, scene, face, hx, ground - face.h as i32, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn town_labels_and_decorations_show() {
        let (mut w, mut d) = crate::game::demo::party(1, &crate::config::Config::default().classes);
        d.advance(66, &mut w);
        let area = Rect::new(0, 0, 120, 12);
        let mut buf = Buffer::empty(area);
        town_scene(&w, None, area, &mut buf, true);
        let ground: String = (0..area.width)
            .map(|x| buf[(x, 11)].symbol().to_string())
            .collect();
        for name in ["src", "assets", "tests"] {
            assert!(ground.contains(name), "{name} missing from {ground:?}");
        }
    }
}
