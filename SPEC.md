# herdr fantasy — Product & Technical Spec

> A herdr plugin that turns your running coding agents into a fantasy RPG party.
> While agents work, they fight monsters, open chests, and battle bosses that
> mirror what they are really doing. When an agent needs you, it raises its hand
> and talks to you through a JRPG dialogue box. Every repo grows its own party,
> quest log, and town over time.

Status: v1 spec · Author: KantaKan · License: MIT · Language: Rust
Companion docs: `CONTEXT.md` (glossary), `docs/adr/` (decisions), `ART.md` (art bible), `TASKS.md` (milestones), `CLAUDE.md` (agent rules)

---

## 1. Goals and non-goals

### Goals
1. **Ambient, not distracting.** It lives in a herdr pane all day. It must never steal focus, input, or noticeable CPU.
2. **Truthful.** Every animation maps to a real agent event. No random "busy" animations pretending to be work.
3. **Zero-config start.** Install the plugin, open herdr, the game is there. Repo, theme, and party are detected automatically.
4. **Fine everywhere, pretty where possible.** Halfblock pixel art is the primary renderer and works in every truecolor terminal, including Windows Terminal. Kitty graphics (true pixel sprites) are an upgrade for Ghostty / kitty / WezTerm.
5. **Persistent story per repo.** Levels, stats, quest log, and town belong to the project and grow with it.
6. **Funny.** Summons in Joke Classes, a Rubber Duck, and Quips — but every joke fires on a real event.
7. **Cross-platform.** Linux, macOS, and native Windows (herdr ships a Windows build).

### Non-goals (v1)
- Prebuilt binaries / install-time downloads. Users need a Rust toolchain (`[[build]]` runs cargo).
- Our own Claude Code hooks. We read agent Transcripts instead (ADR-0001).
- Sixel graphics.
- Answering an agent's question from the popup (risk of sending keys to the wrong prompt). v1 only jumps you to the pane.
- Any network access, telemetry, accounts, payments, or cloud sync.
- Using Anthropic, OpenAI, or any vendor logos or mascots, or any copyrighted characters (no Batman, Joker, Spider-Man, etc. — archetypes only). All characters are original (see `ART.md`).

---

## 2. Glossary

See `CONTEXT.md`. Use its terms exactly (Hero vs Agent, Chapter, Summon, Quip, …).

---

## 3. Integration surface (facts the implementation relies on)

All herdr facts below come from herdr docs v0.9.x (https://herdr.dev/docs/). Re-check the
schema with `herdr api schema --json` at implementation time and fix this section if anything differs.

### 3.1 herdr plugin system
- A plugin is a directory with `herdr-plugin.toml`. Rust binaries are allowed.
- No SDK: the whole herdr CLI is the API. Call it through `$HERDR_BIN_PATH`.
- Manifest supports `[[build]]`, `[[startup]]`, `[[actions]]`, `[[events]]`, `[[panes]]`, `[[link_handlers]]`. Every entry accepts an optional `platforms = ["linux","macos","windows"]` filter (verified in `herdr api schema --json`, herdr 0.9.3).
- Pane placements: `overlay`, `popup`, `split`, `tab`, `zoomed`. Popups capture **all** input until they exit.
- Injected env: `HERDR_SOCKET_PATH`, `HERDR_BIN_PATH`, `HERDR_PLUGIN_ID`, `HERDR_PLUGIN_ROOT`, `HERDR_PLUGIN_CONFIG_DIR`, `HERDR_PLUGIN_STATE_DIR`, `HERDR_PLUGIN_CONTEXT_JSON`, and when available `HERDR_WORKSPACE_ID`, `HERDR_TAB_ID`, `HERDR_PANE_ID`.
- Startup hooks are one-shot (not daemons). They run after session restore when the API socket is ready.
- Plugins cannot register keybindings; users add `[[keys.command]] type = "plugin_action"` to their herdr config.
- Marketplace: public GitHub repo with topic `herdr-plugin` and a parsable manifest.

### 3.2 herdr socket API
- Newline-delimited JSON over a Unix-domain socket (`~/.config/herdr/herdr.sock`; on Windows `%APPDATA%\herdr\herdr.sock`, verified with `herdr status server` on 0.9.3; always prefer `$HERDR_SOCKET_PATH`). On Windows this is AF_UNIX, which tokio does not support natively — use a crate such as `uds_windows` or shell out to `$HERDR_BIN_PATH`. ⚠️ pick and verify at M2.
- Request: `{"id":"...","method":"...","params":{...}}`. Response echoes `id`.
- Key methods: `agent.list`, `agent.get`, `agent.focus`, `pane.read`, `pane.list`, `pane.report_metadata`, `notification.show`, `events.subscribe`, `session.snapshot`, `plugin.pane.open`.
- Agent status values: `idle`, `working`, `blocked`, `done`, `unknown` (`done` = idle and not yet seen).
- `AgentInfo` exposes `agent`, `display_agent`, `terminal_title` and `terminal_title_stripped` (spinner glyph removed), `cwd`, `foreground_cwd`, pane `tokens`, and **`agent_session`** (`{source, agent, kind, value}`) — for Claude, `value` is the transcript path (verified in schema, herdr 0.9.3).
- `events.subscribe` with `pane.agent_status_changed`, `pane.created`, `pane.closed`, `pane.updated`, `pane.focused`, `tab.focused`, `workspace.focused`, `layout.updated`.
- **Events are invalidation signals, not truth.** On every relevant event, re-read with `agent.list` (debounced). On error `events_lost`: resubscribe, wait for `subscription_started`, then `session.snapshot` and replace cache.
- `pane.report_metadata` accepts up to 16 token keys per report, 32 per pane, values ≤ 80 chars, names `[A-Za-z0-9_-]{1,32}`, optional `ttl_ms` and `seq`.
- Panes support the **Kitty graphics protocol**; herdr handles placement, clipping, visibility. The old socket graphics overlay API is removed — emit Kitty escapes in our own pane output.

### 3.3 Agent Transcripts (for rich events) — ADR-0001
- herdr's built-in integrations (`herdr integration install claude`) run a `SessionStart` hook that calls `herdr pane report-agent-session --agent-session-id … --agent-session-path <transcript_path> --session-start-source <startup|resume|clear|compact|fork>`. We read that back from `AgentInfo.agent_session`. We install **no hooks of our own**.
- The Claude transcript is JSONL. Entries we use (⚠️ verify every field name against a real transcript at M4 and update this list):
  - user message → new Quest, title = first line of the prompt text.
  - assistant `tool_use` blocks: `id`, `name` (`Read`, `Edit`, `Bash`, `PowerShell`, `Task`/`Agent`, …), `input` (`file_path`, `command`, …).
  - user `tool_result` blocks: `tool_use_id`, `is_error`.
  - assistant `message.usage`: `input_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`, `output_tokens`.
  - compaction marker (`compact_boundary` / summary entry) → Rest.
- The transcript may lag the live conversation by one flush; that is accepted.
- A change of `agent_session.value` for a pane = new session → new Chapter.
- v1 parses Claude transcripts only. Other agents get status-level animations; the parser emits agent-neutral `GameEvent`s so Codex (`~/.codex/sessions/*.jsonl`) can be added later.

---

## 4. Architecture

Single binary `herdr-fantasy` with subcommands:

```
herdr-fantasy stage            # main pane (long-running TUI)
herdr-fantasy ask              # JRPG popup for blocked agents (short-lived)
herdr-fantasy autostart        # [[startup]] + workspace.created event: open stage if missing
herdr-fantasy open-ask         # action: open the ask popup for the most recently blocked hero
herdr-fantasy toggle-stage     # action: show/hide the stage
herdr-fantasy stage --demo     # scripted fake party, works outside herdr
herdr-fantasy doctor           # print detected terminal graphics, socket, repo, config
```

### 4.1 Module layout

```
src/
  main.rs            # clap subcommands
  config.rs          # load/validate config.toml (defaults baked in)
  herdr/
    client.rs        # socket client: request/response + subscription stream
    types.rs         # serde types for AgentInfo, PaneInfo, events (unknown fields ignored)
    cache.rs         # authoritative cache, debounced refetch, events_lost recovery
  transcript/
    tail.rs          # follow a JSONL file by byte offset; handle truncation/replacement
    claude.rs        # Claude transcript entries → GameEvent (pure, fixture-tested)
  game/
    world.rs         # party, heroes, monsters, current scene
    hero.rs          # per-hero state machine (see §6)
    quest.rs         # quest lifecycle + quest log records
    xp.rs            # XP/level/stats formulas (§7) — pure functions, unit tested
    boss.rs          # classify commands into boss types, resolve outcomes
    summon.rs        # subagent → Summon lifecycle, Joke Class roll (seeded per tool_use id)
    duck.rs          # Rubber Duck triggers + Quip selection, rate limit
    repo.rs          # repo identity, language theme, git stats (cached)
    persist.rs       # save/load per-repo state (atomic writes)
    demo.rs          # scripted scenarios, seeded RNG
  render/
    mod.rs           # frame loop, adaptive FPS, visibility gating
    backend.rs       # graphics capability detection + override
    sprites.rs       # load PNG sheets + JSON; produce Kitty images and halfblock grids
    kitty.rs         # Kitty protocol: transmit once by image id, place/replace by placement id
    halfblock.rs     # ▀ truecolor rendering with transparency
    effects.rs       # tachyonfx transitions, hit flash, floating numbers, shake
    layout.rs        # stage layouts: full / compact / mini
    scenes/
      battle.rs
      town.rs
      questlog.rs
      ask.rs
assets/
  sprites/*.png + *.json   # Aseprite exports (see ART.md)
  font/digits.png          # 3x5 pixel digits for damage/EXP numbers
herdr-plugin.toml
```

### 4.2 Data flow

```
herdr server ──events──▶ herdr::client ──▶ herdr::cache (refetch agent.list, debounce 150ms)
                                               │  (agent_session.value = transcript path)
Claude transcript JSONL ──tail──▶ transcript::claude ──GameEvent──┤
                                               ▼
                                         game::world  (tick 1 logic step per frame)
                                               │
                                               ▼
                                     render (12–15 fps active, 4 fps idle, 0 hidden)
```

- One rich-event channel: the Stage tails each live Claude Agent's transcript (§11). No hook process, no IPC socket, no pane tokens.
- All socket and file reads use tokio, off the render task; render loop reads a snapshot of `world` (no locks held during draw).

### 4.3 Manifest

```toml
id = "kantakan.herdr-fantasy"
name = "herdr fantasy"
version = "0.1.0"
min_herdr_version = "0.9.0"
description = "Your coding agents as an RPG party"
platforms = ["linux", "macos", "windows"]

[[build]]
command = ["cargo", "build", "--release", "--locked"]

[[startup]]
command = ["./target/release/herdr-fantasy", "autostart"]

[[events]]
on = "workspace.created"
command = ["./target/release/herdr-fantasy", "autostart"]

[[actions]]
id = "open-ask"
title = "Talk to agent that needs you"
contexts = ["workspace"]
command = ["./target/release/herdr-fantasy", "open-ask"]

[[actions]]
id = "toggle-stage"
title = "Show/hide the party"
contexts = ["workspace"]
command = ["./target/release/herdr-fantasy", "toggle-stage"]

[[panes]]
id = "stage"
title = "herdr fantasy"
placement = "split"
command = ["./target/release/herdr-fantasy", "stage"]

[[panes]]
id = "ask"
title = "An agent needs you"
placement = "popup"
width = "70%"
height = 18
command = ["./target/release/herdr-fantasy", "ask"]
```

⚠️ Verify at implementation time: exact `herdr plugin pane open` CLI flags for split direction/ratio, whether plugin panes are identifiable in `pane.list` (for "is the stage already open?"), and what happens to plugin panes after a server restart.

---

## 5. Screens and UX

### 5.1 Auto-open ("you enter the game when you open herdr")
- `autostart` runs on server startup and on `workspace.created`.
- If `config.auto_open = "every_workspace"` (default) and that workspace has no stage pane, open one as a **bottom split, ~25% height, min 10 rows**, without taking focus.
- `auto_open = "first_workspace"` or `"off"` are supported.
- Never open twice. Never move focus. If the workspace is too small (< 24 rows total), skip and log.

### 5.2 Stage layout (full mode)

```
┌ herdr fantasy · gymnimal ·  Forge Village ───────────── Party Lv.9 ─┐
│ ⚔ Quest: fix login redirect bug                         ⏱ 04:12      │
│                                                                      │
│   [Knight]     [Mage]                         ┌────┐                 │
│    ▄██▄         ▄██▄    ─✦→   ●●  ●●          │BOSS│ Test Golem      │
│    █▓▓█  ✎      █▓▓█          slimes          │    │ HP ██████░░     │
│    ▀  ▀         ▀  ▀                          └────┘                 │
│ ─────────────────────────────────────────────────────────────────── │
│ Claude  Lv7 ▓▓▓▓▓▓░░ XP  STA ▓▓▓▓░ 62%  editing src/auth.rs        │
│ Codex   Lv4 ▓▓░░░░░░ XP  STA  ?        ❗ needs you   [prefix+a]   │
└──────────────────────────────────────────────────────────────────────┘
```

- Mode is chosen from what fits (measured at M1): chrome = top bar + quest line + one footer row per drawn hero (max 4). **Full** needs chrome + 9 scene rows (16 px hero = 8 rows + ground) and ≥ 40 cols — 14 rows for 3 heroes. **Compact** needs chrome + 5 scene rows (8 px face = 4 rows + ground) and ≥ 30 cols — 10 rows for 3 heroes. Otherwise **Mini**. Heroes drawn = what fits the width (17 cols each in full, 10 in compact), rest as `+N`.
- Top bar: repo name, town theme name, party level (sum of hero levels / count, rounded). Each Hero carries its own repo's progress; if the Party spans several repos, the top bar shows the repo with the most Heroes (ties → most recently active).
- Quest line: current quest title of the **focused or most recently active** hero.
- Scene: heroes left, enemies right. Max 4 heroes drawn; extra shown as `+N`. Up to 3 Summons drawn small beside their Hero, extra as `+N`. The Rubber Duck sits by the campfire.
- Footer: one row per hero: name, level, XP bar, stamina bar, current activity text.
- Keys inside stage (only when stage pane is focused): `t` town, `q` quest log, `c` character sheet of selected hero, `←/→` select hero, `Enter` focus that agent's pane, `?` help, `Esc` back to battle.

### 5.3 Compact mode and mini mode (thresholds in §5.2)
- Compact: smaller scene (heroes as 8×8 faces), footer kept. The boss isn't drawn; its name + HP stay on the quest line.
- Mini: no scene. One line per hero: face glyph, state icon, XP bar, activity. Still animates the face glyph.
- Width < 50 cols: drop stamina bar text, then activity text.

### 5.4 Hero states → what you see

| Source | Trigger | Animation |
|---|---|---|
| herdr | `idle` | sits down (legs forward), slow breathing, floating `z`s; when the **whole party** is idle and no boss is up, the slimes leave and a campfire with the Rubber Duck appears |
| herdr | `working` (no rich events) | walks forward, auto-battles slimes; small XP ticks |
| herdr | `blocked` | stops, `❗` bubble above head, bounces gently; footer shows key hint |
| herdr | `done` (not yet seen) | victory pose + sparkle loop until seen |
| herdr | pane closed / agent gone | hero walks off-screen to the left |
| transcript | new user prompt | new quest banner slides in (tachyonfx sweep) |
| transcript | `Read` / `Grep` / `Glob` / `LS` | walks to a chest, opens it, item icon pops |
| transcript | `WebSearch` / `WebFetch` | raises spyglass, scanning sparkle |
| transcript | `Edit` / `Write` / `MultiEdit` / `NotebookEdit` | one sword swing at nearest slime; damage number = lines changed (clamped 1–999) |
| transcript | `Bash` / `PowerShell` (not a boss command) | casts spell, rune circle effect |
| transcript | `Bash` / `PowerShell` boss command (§6.3) | boss enters screen shake; battle until its `tool_result` |
| transcript | `Task` / `Agent` (subagent) start | **Summon** rises from shadow (§6.5); sinks back on its `tool_result` |
| transcript | context ≥ 70% | "Tired": sweat drop every few seconds |
| transcript | context ≥ 90% | "Exhausted": slower walk cycle, pants; toast once: "Claude is running out of stamina" |
| transcript / herdr | compaction marker, or `agent_session` source `compact` | **Rest scene**: hero sits by fire, screen dims, glowing "memory" particles drift into a journal; then "Well Rested" buff (2 min sparkle), STA refills |
| herdr | `agent_session.value` changes (`/clear` or new session) | **New Run** banner: Chapter closes in quest log, hero keeps level/stats |
| game | Rubber Duck trigger (§6.6) | Duck waddles up, speech bubble with a Quip |

Rules:
- Transcript events queue per hero; play at most one action animation at a time; if the queue exceeds 6, collapse into a combo ("x7 slashes!").
- herdr status always wins over transcript-derived state for idle/working/blocked/done.
- Agents without a parsed transcript (Codex, Cursor, etc. in v1) get only the herdr-status rows. Stamina shows `?`.

### 5.5 Ask popup ("agent needs you")
- Never opens by itself. When any hero is blocked: `❗` on the hero, plus one `herdr notification show` with `sound = "request"` (configurable).
- User opens it with the `open-ask` action (suggested binding `prefix+a`; documented in README).
- Contents (JRPG box, see ART.md §6):
  - Left: hero portrait (32×32, two-frame talk animation).
  - Right: agent name + dialogue text: `pane.read --source detection` last ~12 lines, trimmed, ANSI stripped.
  - Text appears with a typewriter effect (skip with any key).
  - Options: `▶ Go to this agent` (`agent.focus`, then exit), `Next agent` (if several are blocked), `Close`.
- Popup must exit on `Esc`, `q`, or after focusing. It must never send keys to agents in v1.

### 5.6 Quest log (`q`)
- Chapters per repo. A chapter = one agent session (ends on New Run or SessionEnd).
- Each quest row: title, hero, duration, XP gained, bosses fought (✓/✗), result.
- Quest title priority: transcript user prompt (first line, ≤ 60 chars) → `terminal_title_stripped` → "Untitled quest".
- `config.store_prompt_text = true` by default; when false, store only the title source "prompt" and a hash. Everything stays local.

### 5.7 Town (`t`)
- The repo as a village. Up to 6 buildings = top-level directories ranked by tracked file count (`git ls-files`, cached, refreshed on `Stop` at most once per 60 s).
- Building size tier grows with that directory's file count; town gets decorations (trees, lanterns, a statue) as total commit count crosses thresholds (10, 50, 100, 250, 500, 1000).
- Heroes stand next to the building matching their last touched file (`file_path` of the latest file tool in the transcript).
- The Town shown follows the selected Hero's repo.
- Theme by dominant language (file extension counts): Rust → **Forge Village**, TS/JS → **Lantern City**, Go → **Harbor Town**, Python → **Serpent Grove**, other → **Meadow Hamlet**.

### 5.8 Character sheet (`c`)
Portrait, class, level, XP to next, stats (§7.3), lifetime quests, bosses won/lost, best streak, rests taken, Shadows raised (lifetime Summons).

---

## 6. Game logic

### 6.1 Repo identity
1. `git -C <cwd> config --get remote.origin.url` → normalize (strip `.git`, scheme, user; lowercase host).
2. else `git rev-parse --git-common-dir` resolved to an absolute path (so worktrees share one identity).
3. else the cwd itself (non-git folder).
Key = SHA-256 of the identity string, first 16 hex chars. Cache per cwd for the stage's lifetime.

### 6.2 Hero identity and class
- Hero key = `(repo_key, agent_kind, slot)`. `slot` = 0-based order among live agents of the same kind in the same repo, assigned when the agent appears and freed when it leaves. Reappearing agents take the lowest free slot.
- Default classes (configurable): `claude → Knight`, `codex → Mage`, `cursor → Rogue`, `opencode → Ranger`, `gemini → Cleric`, anything else → `Bard`.
- Display name = herdr agent label (e.g. "Claude"), plus ` II`, ` III` for slots > 0.

### 6.3 Bosses
Classify `Bash` / `PowerShell` commands from the transcript `tool_use.input.command` (first matching rule wins; rules configurable):

| Boss | Matches (substring/regex on the command) |
|---|---|
| **Test Golem** | `cargo test`, `cargo nextest`, `npm test`, `pnpm test`, `yarn test`, `bun test`, `vitest`, `jest`, `pytest`, `go test`, `just test`, `make test` |
| **Build Wyrm** | `cargo build`, `cargo check`, `tsc`, `npm run build`, `pnpm build`, `go build`, `make`, `vite build`, `next build` |
| **Lint Imp** | `cargo clippy`, `eslint`, `biome`, `ruff`, `golangci-lint`, `prettier --check` |

- **Species**: each kind has a roster; the species is a stable hash (FNV-1a) of the command text, so the same command always summons the same nemesis.

| Kind | Roster |
|---|---|
| Test | Test Golem, Flaky Ghost, Snapshot Mimic |
| Build | Build Wyrm, Linker Kraken, Dependency Hydra |
| Lint | Lint Imp, Typo Gremlin, Off-by-One Slime King |
| Any (rare, 1 in 8 commands) | **Sahur Knocker** — a wooden kentongan log with a bat; original, inspired by the sahur wake-up drum (not the meme character) |

- **Size tier** follows the command's scope: whole-repo flags (`--workspace`, `--all`, `--all-targets`, `--all-features`, `--recursive`, `-r`) → **Big**; a narrowed run (a token with `::` or `/`, a source file, or `-k` / `--test` / `-run` / `-t` / `--grep`) → **Little**; otherwise **Normal**. If the tier doesn't fit the scene height, it drops a tier.
- The quest line shows the boss name + HP while a boss is present (instead of the quest timer).
- Outcome: matching `tool_result` with `is_error = false` = victory, `is_error = true` = boss survives and laughs. ⚠️ verify that a non-zero exit sets `is_error` for both Bash and PowerShell; if not, parse the exit code from the result text.
- Boss HP is cosmetic: starts full; each failed attempt knocks off 25% (so persistence visibly pays off); victory always finishes it.
- "First-try" = victory without a failure on the same boss type within the same quest.

### 6.4 Quest lifecycle
- Start: a transcript user prompt, or herdr status → `working` with no open quest.
- End: herdr status → `idle`/`done` and no new prompt within 10 s.
- Abandon: pane closes during a quest → recorded as "unfinished".

### 6.5 Summons and Joke Classes
- A subagent `tool_use` (`Task` / `Agent`) spawns a **Summon** next to the Hero that launched it; its `tool_result` dismisses it. Several may run in parallel.
- Summons have no level and are not saved. Any XP they earn goes to the summoning Hero (same per-quest caps). Each spawn increments the Hero's lifetime "Shadows raised".
- Look: a `p`/`k` shadow rises from the ground, solidifies into a **Joke Class** costume, fights, then sinks back into shadow.
- Joke Class = seeded pick from the table below using the subagent's `tool_use` id (stable for its lifetime, reproducible in demo):

| Joke Class | Gimmick |
|---|---|
| **10x Engineer** | ten headphones; hits for 10× but misses half the time |
| **Jester** | chaos clown; random spell visuals |
| **Night Owl** | caped vigilante debugger; buffed after local midnight |
| **Intern** | cardboard crown; trips over slimes, brings coffee |
| **Stack Overflow Pilgrim** | giant scroll of copied answers; casts spells it doesn't understand |
| **Vibe Coder** | sunglasses + headphones; attacks by vaguely gesturing |
| **Senior Dev** | grey beard, coffee; "it depends"; one-shots one slime per quest |
| **YAML Wizard** | robe of indentation; spell fizzles off-by-one-space |
| **Product Manager** | points at enemies with a roadmap while others fight |
| **On-Call Paladin** | armor covered in pager alerts; half asleep |
| **Rubber Duck Cultist** | worships the Duck; buffed when it quips |
| **Merge Conflict Twins** | two halves of one sprite arguing `<<<<<<<` / `>>>>>>>` |

- Gimmicks are cosmetic only; they never change real XP.

### 6.6 Rubber Duck and Quips
- One Rubber Duck per Stage, by the campfire. Never levels, never played by an Agent.
- Triggers (each from real state): a Boss survives ≥ 2 times in a row; a Hero blocked ≥ 5 min; a Quest running ≥ 30 min. At most one Duck Quip per 2 min.
- Quips are original, text-only lines (no images, no quotes from copyrighted works) in a built-in table keyed by event, e.g. Boss defeated after ≥ 3 failures → "it works on my machine"; `git push --force` → "no fear"; Sahur Knocker laughs → "TUNG TUNG TUNG!"; Rest at ≥ 95% context → "I have no memory of this place".
- `memes = false` disables all Quips; the Duck still sits there.

---

## 7. Progression

All formulas live in `game/xp.rs` as pure functions with unit tests. Constants in config.

### 7.1 XP sources
| Source | XP |
|---|---|
| Context growth (Claude, from transcript usage) | `+1 XP per 1,000 new context tokens` (delta of current context size; negative deltas ignored) |
| Output tokens (Claude) | `+1 XP per 200 output tokens` |
| Working time (agents without a parsed transcript) | `+1 XP per 10 s working`, capped at 60 XP per quest from time |
| Quest complete | `+25` |
| Boss victory | `+40`, first-try bonus `+30` |
| Boss defeated after ≥ 2 failures ("Persistence") | `+20` extra |
| Rest survived (compaction) | `+10` |

Anti-farm:
- Time XP rate halves after 20 min of continuous working in one quest, and stops at the per-quest cap.
- Token XP per quest is capped at 400; beyond that, quests still give completion/boss XP.
- Nothing is earned while a hero is `blocked` or `idle`.

### 7.2 Levels
- XP needed for next level: `round(100 * level^1.5)`. Level 1 start. No max level.
- Level-up: golden column of light, `LEVEL UP!` pixel text, +1 stat point auto-assigned to the hero's top stat.

### 7.3 Stats (shown on character sheet; flavor + small cosmetic effects)
| Stat | Grows from |
|---|---|
| **STR** | every 10 edit actions |
| **INT** | every 10 read/search/web actions |
| **DEX** | every 10 Bash actions |
| **VIT** | every rest (compaction) survived |
| **LUK** | every first-try boss victory |

### 7.4 Stamina (context window) — "what happens when context is full"
- `STA% = 100 - context_pct`, where `context_pct = current_context_tokens / context_window_size`.
- `context_window_size`: config map by model id, default 200,000; if usage exceeds the configured size, grow the assumed size to the next known tier (200k → 1M) instead of showing > 100%.
- Thresholds: ≥ 70% used → Tired; ≥ 90% → Exhausted (one toast per quest).
- Compaction is **not** a punishment: it is the Rest scene, gives VIT and XP, and refills STA. XP and level are never lost.
- `/clear` → New Run: STA full, new chapter, level kept.
- Agents with unknown context: STA shows `?` and no stamina states.

---

## 8. Rendering

### 8.1 Backend selection
- `config.render = "auto" | "kitty" | "halfblock"`; `auto` probes Kitty graphics support inside the pane (query `a=q`, short timeout) and falls back to halfblock.
- `doctor` prints the detected backend so users can report issues.
- Over SSH / remote herdr: same probe; if it fails, halfblock. Must work in both.
- Windows Terminal has no Kitty graphics: the probe fails and halfblock is used. That is the expected primary path on Windows.

### 8.2 Kitty path
- Load each sprite sheet once; transmit each frame once with a stable image id at startup.
- Animate by changing placements (placement id per actor), never by retransmitting pixels.
- Scale sprites by an integer factor to fit cell size; nearest-neighbor only.
- On resize / tab switch / pane hidden: delete our placements, re-place when visible.

### 8.3 Halfblock path
- `▀` with fg = top pixel, bg = bottom pixel, truecolor. Transparent pixels show the scene background color, not the terminal default.
- Same PNG sources as Kitty (single asset pipeline).

### 8.4 Effects
- Hit flash (1 white frame), floating numbers (3×5 pixel digits, rise 3 rows over 600 ms, fade), screen shake (±1 cell, 150 ms, boss hits only), squash/stretch on jumps, tachyonfx for scene transitions and the ask box opening.
- Respect `config.reduced_motion = true`: no shake, no flashes, transitions become instant.

### 8.5 Frame loop and performance budget
- Adaptive FPS: 12–15 during actions/battles, 4 when all heroes idle, **0 when the stage pane is not visible** (other tab/workspace focused, or zoomed away).
- Only redraw on change (ratatui diffing; skip frame if world unchanged).
- Budget (Apple Silicon / modern Linux, release build):
  - CPU: < 1% idle, < 3% in battle, measured over 60 s.
  - RSS: < 30 MB.
  - Startup to first frame: < 150 ms.
  - Transcript tailing: only appended bytes read; never on the render task.
- Add a `--bench` flag to `stage --demo` that prints average CPU and frame time after 60 s.

---

## 9. Config (`$HERDR_PLUGIN_CONFIG_DIR/config.toml`)

```toml
auto_open = "every_workspace"   # every_workspace | first_workspace | off
render = "auto"                 # auto | kitty | halfblock
reduced_motion = false
memes = true                    # Quips from the Rubber Duck and Summons
notify_on_blocked = true
notify_sound = "request"        # none | done | request
store_prompt_text = true
show_all_workspaces = false     # stage shows only current workspace's party

[classes]
claude = "Knight"
codex = "Mage"
cursor = "Rogue"
opencode = "Ranger"
gemini = "Cleric"
default = "Bard"

[context_window]
default = 200000

[bosses]
# extra user rules, appended before built-ins
test = []
build = []
lint = []
```

Invalid config: log a warning, fall back to defaults per key, never crash.

---

## 10. Persistence (`$HERDR_PLUGIN_STATE_DIR`)

```
state/
  repos/<repo_key>/
    party.json        # heroes: level, xp, stats, lifetime counters
    quests.jsonl      # append-only quest log
    town.json         # cached git stats + decorations unlocked
  version             # schema version for migrations
```
- Atomic writes (write temp + rename). Save on level-up, quest end, and every 30 s if dirty.
- Corrupt file → move to `*.corrupt-<ts>`, start fresh, log it.
- Only the stage writes state files.

---

## 11. Transcript reading

- For each live Claude Agent, take the path from `AgentInfo.agent_session.value` (`agent == "claude"`). No path → status-only hero.
- Open read-only, never lock, never write. Keep a byte offset per path; read only appended bytes; keep a partial trailing line for the next read.
- File shrank or was replaced → reset offset to 0. Path changed → close the Chapter and start a new one.
- On first sight of a path: read the last 64 KB to find the latest usage entry for Stamina, then follow from the end (no replay of history as animations).
- Wake on file-change notifications if reliable on the platform, else poll every 250 ms while the hero is `working`, every 2 s otherwise.
- Malformed lines are skipped (debug log). Never log prompt text at info level.

---

## 12. Demo mode

`herdr-fantasy stage --demo [--scenario NAME] [--seed N] [--speed X]`
- Runs without herdr (no socket needed). Same renderer.
- Built-in scenarios:
  - `party` (default): 3 heroes; reads, edits, a Test Golem that fails twice then dies (Duck quips), two Summons rise and sink, one hero blocked with a dialogue, a compaction rest, a level-up, quest complete.
  - `boss`: boss fight loop for tuning effects.
  - `town`: town view with growth over time.
  - `stress`: 8 agents, max event rate, for perf testing.
- Deterministic with `--seed` so GIFs and snapshot tests are reproducible.
- `herdr-fantasy ask --demo` shows the popup with fake text.

---

## 13. Testing

- Unit: xp formulas, level curve, boss classifier, repo identity normalization, hero state machine transitions, Joke Class roll, Duck triggers + rate limit, transcript parsing → GameEvent (fixtures from a real transcript).
- Protocol: serde round-trip tests with captured herdr JSON fixtures in `tests/fixtures/` (record them with `herdr api snapshot` and a subscription dump).
- Render: snapshot tests of halfblock frames using ratatui `TestBackend` + `insta`, driven by demo scenarios at fixed seeds/ticks.
- Transcript tail: append to a temp file in chunks (including split lines), truncate it, replace it; assert the right events and no replay.
- Manual QA checklist in TASKS.md (Windows Terminal, Ghostty with Kitty graphics, SSH remote).

---

## 14. Error handling and logging
- `tracing` to `$HERDR_PLUGIN_STATE_DIR/logs/herdr-fantasy.log`, daily rotation, keep 3. Never log prompt text at info level.
- Socket disconnect: show a small "reconnecting…" banner, retry with backoff (250 ms → 5 s), keep last world state on screen.
- Any panic in render: restore terminal state before exit.

---

## 15. Open questions to verify during implementation
1. Exact CLI flags for `herdr plugin pane open` (split direction, ratio, no-focus). Partial: `herdr pane split` has `--direction right|down`, `--ratio <f>` (sizes the new pane), `--focus` / `--no-focus`, `--cwd`, `--env`; check whether `plugin pane open` takes the same.
2. How to detect an existing stage pane (plugin ownership in `pane.list`?).
3. Behavior of plugin panes after herdr server restart / live handoff.
4. Whether `agent_session` updates emit `pane.updated` / `pane.agent_status_changed` (otherwise poll `agent.list` every 2 s while any hero is working).
5. Kitty graphics probe results inside herdr panes, locally and over SSH.
6. Claude transcript field names (tool_use, tool_result `is_error`, usage, compaction marker, subagent tool name `Task` vs `Agent`), and whether a non-zero exit sets `is_error` for Bash and PowerShell.
7. Herdr agent `agent` field values for each supported agent (for class mapping). Integrations on herdr 0.9.3: pi, omp, claude, codex, copilot, devin, droid, kimi, opencode, kilo, hermes, qodercli, qwen, cursor, mastracode, antigravity-cli, grok, letta.
8. Windows: AF_UNIX client for the herdr socket (`uds_windows` vs shelling out to `herdr api`), and whether file-change notifications on transcripts are reliable.

### Resolved
- herdr runs natively on Windows (0.9.3); plugin `platforms` enum = `linux | macos | windows`; socket `%APPDATA%\herdr\herdr.sock`.
- `AgentInfo.agent_session` carries the transcript path, reported by herdr's own integration hook.
- Windows: `./target/release/herdr-fantasy` in the manifest runs the `.exe` (verified with `herdr plugin action invoke kantakan.herdr-fantasy.open-ask`, exit 0) — no per-platform manifest entries needed.
- Windows paths herdr injects: config `%APPDATA%\herdr\plugins\config\<plugin-id>`, state `%LOCALAPPDATA%\herdr\plugins\<plugin-id>`.
- `herdr plugin action invoke` takes the fully-qualified id (`<plugin-id>.<action-id>`); `herdr plugin log list` shows exit code and stderr.
