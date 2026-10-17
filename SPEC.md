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
- Plugins cannot register keybindings; users add `[[keys.command]]` with `key = "prefix+a"`, `type = "plugin_action"`, `command = "kantakan.herdr-fantasy.open-ask"` (validated with `herdr config check`; `action =` is rejected).
- `plugin.pane.open`: split/zoomed panes take `target_pane_id` and reject `workspace_id`; overlay/popup panes open over the active pane and reject `workspace_id` too. A popup open returns only `ok` (no pane id). `pane.resize {direction, amount}` moves the split ratio by `amount` (0.25 on a fresh 50/50 split → 75/25).
- Marketplace: public GitHub repo with topic `herdr-plugin` and a parsable manifest.

### 3.2 herdr socket API
- Newline-delimited JSON over a Unix-domain socket (`~/.config/herdr/herdr.sock`; always prefer `$HERDR_SOCKET_PATH`). **On Windows it is a named pipe**: herdr serves `\\.\pipe\<socket path>` (e.g. `\\.\pipe\C:\Users\me\AppData\Roaming\herdr\herdr.sock`), and the file at the socket path only holds `pid:nonce`. Open the pipe like a file (`OpenOptions` read+write; retry on `ERROR_PIPE_BUSY`). **The pipe closes after each response**, so the client opens one connection per request (on every platform; local connects are cheap). Verified on 0.9.3.
- Request: `{"id":"...","method":"...","params":{...}}`. Response echoes `id`.
- Key methods: `agent.list`, `agent.get`, `agent.focus`, `pane.read`, `pane.list`, `pane.report_metadata`, `notification.show`, `events.subscribe`, `session.snapshot`, `plugin.pane.open`.
- Agent status values: `idle`, `working`, `blocked`, `done`, `unknown` (`done` = idle and not yet seen).
- `AgentInfo` exposes `agent`, `display_agent`, `terminal_title` and `terminal_title_stripped` (spinner glyph removed), `cwd`, `foreground_cwd`, pane `tokens`, and **`agent_session`** (`{source, agent, kind, value}`, `kind` = `id` | `path`). Real Claude panes report `kind: "id"` with the session UUID, not the path (verified 0.9.3).
- `events.subscribe` exists, but `pane.agent_status_changed` subscriptions require a `pane_id` each (verified 0.9.3), so there is no single "any agent changed" stream.
- **v1 polls `session.snapshot`** (~6 KB: agents, focused workspace, each workspace's active tab, layouts with zoom) every 200 ms while the stage is visible, 1 s while hidden, on one persistent connection. Changes only are sent to the render loop. Reconnect with backoff 250 ms → 5 s and a "reconnecting…" banner.
- `pane.report_metadata` accepts up to 16 token keys per report, 32 per pane, values ≤ 80 chars, names `[A-Za-z0-9_-]{1,32}`, optional `ttl_ms` and `seq`.
- Panes support the **Kitty graphics protocol**; herdr handles placement, clipping, visibility. The old socket graphics overlay API is removed — emit Kitty escapes in our own pane output.

### 3.3 Agent Transcripts (for rich events) — ADR-0001
- herdr's built-in integrations (`herdr integration install claude`) run a `SessionStart` hook that calls `herdr pane report-agent-session --agent-session-id … --agent-session-path <transcript_path> --session-start-source …`. We read that back from `AgentInfo.agent_session`. We install **no hooks of our own**.
- `agent_session.kind == "path"` → `value` is the transcript. `kind == "id"` (what Claude panes report on 0.9.3) → the transcript is `~/.claude/projects/<slug(cwd)>/<value>.jsonl`, where `slug` replaces every non-alphanumeric character of the agent's `cwd` with `-` (`D:\code\hrdr-game` → `D--code-hrdr-game`). Verified against real files.
- The Claude transcript is JSONL. Entries we use (**verified against real transcripts at M4**; fixture `tests/fixtures/claude_transcript.jsonl`, scrubbed):
  - prompt: `type:"user"` with `message.content` a **string**. Skip `isMeta: true`, `isCompactSummary: true`, `isSidechain: true`, and strings starting with `<` (command echoes). Text blocks `[Request interrupted…]` are not prompts. Newer Claude Code (verified 2026-10-16) also logs a typed slash command as a plain prompt line (`"/compact"`) before its `<command-name>` echo: a line whose first word is `/name` is not a prompt.
  - tool call: `type:"assistant"`, `message.content[]` block `{type:"tool_use", id, name, input}`. Names seen: `Read`, `Edit`, `Write`, `Grep`, `Glob`, `Bash`, `PowerShell`, `Skill` (`input.skill`), `Agent` (subagents: `input.subagent_type`, `description`, `prompt`, `run_in_background`), `AskUserQuestion`, `mcp__*`.
  - tool result: `type:"user"`, block `{type:"tool_result", tool_use_id, is_error?}`. A non-zero Bash/PowerShell exit **does** set `is_error: true` (content starts `Exit code N`).
  - usage: `message.usage` on assistant lines; context = `input_tokens + cache_creation_input_tokens + cache_read_input_tokens`; `message.model` names the model.
  - compaction: `type:"system", subtype:"compact_boundary"`, `compactMetadata: {trigger, preTokens, postTokens}`, followed by an `isCompactSummary` user line.
  - **Codex** (M17, `transcript/codex.rs`): herdr's Codex integration reports the session id; the rollout is `~/.codex/sessions/YYYY/MM/DD/rollout-<time>-<id>.jsonl` (or `$CODEX_HOME`), found once and cached. Lines are `{timestamp, type, payload}` (verified). ⚠ Payload shapes for prompts (`event_msg/user_message`), shell calls (`function_call` shell/exec_command → Bash), `apply_patch` (→ Edit, file + added lines), outputs (exit code from `metadata.exit_code` or "Exit code: N"), `token_count` (context = last input + output, window from `model_context_window`) and compaction follow Codex's documented rollout format and still need a check against a real session with tool calls.
  - async subagents (verified 2026-10-16, the default now): the `Agent` `tool_result` comes back ~2 s after the call with text `Async agent launched successfully … agentId: <id> …`. The subagent's finish is a later `type:"queue-operation", operation:"enqueue"` line whose `content` starts `<agent-message from="<id>">`.
  - lots of other line types (`attachment`, `mode`, `ai-title`, `file-history-*`, …) are ignored.
- **Measured lag:** Claude Code flushes the transcript in batches; a tool call became readable ~2.3 s after it started (Windows, 2026-10-16). Animations therefore trail the agent by ~2–3 s. Accepted for v1 (ADR-0001).
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
height = 20
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
- Keys inside stage (only when stage pane is focused): `t` town, `q` quest log, `c` character sheet of selected hero, `w` weekly recap, `s` settings, `←/→` select hero (footer `▶`), `↑/↓` scroll the quest log, `Enter` focus that agent's pane (`agent.focus`), `?` help, `Esc` back to battle, `Ctrl-C` quit. Each screen key toggles. Quest log, sheet, recap and help use the whole pane below the top bar in every layout; the town replaces the battle scene.

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
| transcript | `Read` | opens a treasure chest |
| transcript | `Grep` | reads the big book |
| transcript | `Glob` / `LS` | spyglass scout |
| transcript | `WebSearch` / `WebFetch` | raises spyglass, scanning sparkle |
| transcript | `Edit` / `Write` / `MultiEdit` / `NotebookEdit` | one sword swing at nearest slime; damage number = lines changed (clamped 1–999) |
| transcript | `Bash` / `PowerShell` (not a boss command) | casts spell, rune circle effect |
| transcript | `Bash` / `PowerShell` boss command (§6.3) | boss enters screen shake; battle until its `tool_result` |
| transcript | `Task` / `Agent` (subagent) start | **Summon** rises from shadow (§6.5); sinks back on its `tool_result`, or for an async subagent on its `<agent-message>` hand-back |
| transcript | context ≥ 70% | "Tired": sweat drop every few seconds |
| transcript | context ≥ 90% | "Exhausted": slower walk cycle, pants; toast once: "Claude is running out of stamina" |
| transcript / herdr | compaction marker, or `agent_session` source `compact` | **Rest scene**: hero sits by fire, screen dims, glowing "memory" particles drift into a journal; then "Well Rested" buff (2 min sparkle), STA refills |
| herdr | `agent_session.value` changes (`/clear` or new session) | **New Run** banner: Chapter closes in quest log, hero keeps level/stats |
| game | Rubber Duck trigger (§6.6) | Duck waddles up, speech bubble with a Quip |

**Shell commands and skills get their own moves** (M4, from the transcript's `Bash`/`PowerShell` `command` and `Skill` tool calls; first matching rule wins, non-matching commands cast the default spell):

| Command / tool | Animation |
|---|---|
| `grep`, `rg`, `ag`, `findstr`, `Select-String` | sits and reads a big book, pages flipping |
| `cat`, `less`, `head`, `tail`, `type`, `Get-Content` | unrolls a scroll and reads it |
| `ls`, `dir`, `find`, `fd`, `tree`, `Get-ChildItem` | spyglass scout |
| `curl`, `wget`, `Invoke-WebRequest` | releases a messenger bird |
| `git commit` | stamps a wax seal on a scroll |
| `git push` | sends a carrier pigeon flying right |
| `git pull`, `git fetch` | a pigeon lands with a letter |
| `npm/pnpm/yarn/bun install`, `cargo add`, `pip install` | shops at a tiny merchant, coin pops |
| `rm`, `del`, `Remove-Item` | small fireball |
| `mkdir`, `touch`, `cp`, `mv`, `New-Item` | hammers on an anvil |
| `sleep`, `Start-Sleep` | dozes standing up |
| `Skill` tool | opens a glowing spellbook; floating text `learned: <skill name>` |

Implemented in `game/moves.rs` (pure, table-tested). Edits float a damage number (`-N`, lines written, 1–999). Props are 8×8 sprites (`prop_*`, `tools/gen_props.py`) held at the hands, flown off/in, or popped up; heroes use a `hold` pose for held props.

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
- Each quest row: title, hero, duration, XP gained, bosses fought (✓/✗), result. Rows are newest first under `── <hero> · Chapter N ──` headings; `quests.jsonl` lines carry `bosses` and `chapter` (older lines without them read as chapter 1).
- The log is loaded from disk when a repo's first hero arrives, so it survives restarts.
- A new prompt completes the open quest — unless that quest is under 2 s old (it was started by the same prompt): then it's just renamed.
- Quest title priority: transcript user prompt (first line, ≤ 60 chars) → `terminal_title_stripped` → "Untitled quest".
- `config.store_prompt_text = true` by default; when false, store only the title source "prompt" and a hash. Everything stays local.

### 5.7 Town (`t`)
- The repo as a village. Up to 6 buildings = top-level directories ranked by tracked file count (`git ls-files`, cached, refreshed on `Stop` at most once per 60 s).
- Building size tier grows with that directory's file count; town gets decorations (trees, lanterns, a statue) as total commit count crosses thresholds (10, 50, 100, 250, 500, 1000).
- Building tiers: < 20 files, < 100, more (16/20/24 px sprites per theme). Top-level dirs starting with `.` are skipped. Survey = `git ls-files --full-name -- :/` + `git rev-list --count HEAD` on the herdr thread, at most once a minute per repo.
- Decorations stand in the background spread across the town; buildings go in front, labelled on the ground.
- Compact layout: a text town (`Forge Village · N commits` and `[dir]` labels with hero initials).
- Heroes stand next to the building matching their last touched file (`file_path` of the latest file tool in the transcript; any path component naming a building). Others stand in the town square on the left.
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
- Default classes (configurable): `claude → Knight`, `codex → Mage`, `cursor → Rogue`, `opencode → Ranger`, `gemini → Cleric`, anything else → `Bard`. Special classes can be chosen in `[classes]` too: `"Sahur Knight"`, `"Wild Card"` (ART.md §4).
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
- Matching (`game/boss.rs`, table-tested): each `&&` / `;` / `|` segment of the command, after skipping `cd`, env assignments and `sudo`, is compared word-by-word against the rules (`cargo test` matches `cargo test --workspace`, not `cargo testy` or `echo cargo test`). User rules (test, build, lint) first, then built-ins; test rules beat a plain `make`. The built-in tables also cover `npm run test/build/lint`, `dotnet`, `mvn`, `gradle`, `mypy`, `flake8`, `rubocop`, `cargo fmt --check`.
- Outcome: the matching `tool_result` (by `tool_use_id`). `is_error = true` (verified: non-zero exit) = boss survives and laughs. **Agents often pipe through `tail`/`head`, which hides the exit code**, so a result also counts as failed when its text shows `test result: FAILED`, `error[E`, `could not compile`, `npm ERR!`, `--- FAIL`, `FAILED (`, `panicked at`, … (`boss::failed`).
- The same kind of boss stays up between attempts (failures add up, HP keeps dropping); a different kind replaces it. Victory XP goes to the hero who fought it, with a floating `+N XP first try!` / `persistence!`.
- Boss HP is cosmetic: starts full; each failed attempt knocks off 25% (so persistence visibly pays off); victory always finishes it.
- "First-try" = victory without a failure on the same boss type within the same quest.

- **Melee** (v0.2.2): a hero whose boss command is running runs over to the boss (0.5 s, eased) and swings in melee: the 3 attack frames, then the wind-up held, every 0.6 s; each strike frame knocks the boss back a column with a spark at the point of contact. Team fighters line up behind the first. A failed run knocks the hero home (0.8 s, facing back); a win keeps it there for the death animation, then it runs home. Pets run along and do their trick. `reduced_motion` keeps everyone in place.

### 6.4 Quest lifecycle
- Start: a transcript user prompt, or herdr status → `working` with no open quest.
- End: herdr status → `idle`/`done` and no new prompt within 10 s.
- Abandon: pane closes during a quest → recorded as "unfinished".
- Quests are per hero. `blocked` pauses the quest (never ends it); `idle`/`done` start the 10 s end timer. The HUD shows the focused hero's quest, else the most recently active one's, tagged with the hero name when the party has more than one hero.

### 6.5 Summons and Joke Classes
- A subagent `tool_use` (`Task` / `Agent`) spawns a **Summon** next to the Hero that launched it; its `tool_result` dismisses it. Several may run in parallel.
- Summons have no level and are not saved. Subagent transcripts are sidechains we skip, so a Summon's XP is time-based: 1 per 10 s it was out, capped at 30, paid to the summoning Hero when it sinks. Each spawn increments the Hero's lifetime "Shadows raised" (character sheet). An async subagent (result `Async agent launched … agentId: X`) keeps its Summon until the `<agent-message from="X">` hand-back (30 min safety cap). Any other background subagent that returns at once stays 60 s and then sinks.
- Rise and sink take 0.6 s each: the sprite is revealed bottom-up out of a `p`/`k` shadow puddle. Each Joke Class says its entrance line for 2.5 s (a Quip, so `memes = false` hides it). The footer shows `☠N` while Summons are out.
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
- One Rubber Duck per Stage: by the campfire when the party camps, otherwise just in front of the enemies. Never levels, never played by an Agent. Its speech bubble sits top-right of the scene for 5 s (full and compact layouts). Every Duck quip is logged at info level.
- Triggers (each from real state): a Boss survives ≥ 2 times in a row; a Hero blocked ≥ 5 min; a Quest running ≥ 30 min. At most one Duck Quip per 2 min.
- Quips are original, text-only lines (no images, no quotes from copyrighted works) in a built-in table keyed by event, e.g. Boss defeated after ≥ 3 failures → "it works on my machine"; `git push --force` → "no fear"; Sahur Knocker laughs → "TUNG TUNG TUNG!"; Rest at ≥ 95% context → "I have no memory of this place".
- `memes = false` disables all Quips; the Duck still sits there.
- Each occurrence quips once (e.g. one block, one boss fight); a block that ends lets the next one quip again. Implemented in `game/duck.rs` (pure, tested); the Sahur Knocker's "TUNG TUNG TUNG!" shows over the boss while it laughs.

---

## 7. Progression

All formulas live in `game/xp.rs` as pure functions with unit tests. Constants in config.

### 7.1 XP sources
| Source | XP |
|---|---|
| Context growth (Claude, from transcript usage) | `+1 XP per 1,000 new context tokens` (delta of current context size; negative deltas ignored) |
| Output tokens (Claude) | `+1 XP per 200 output tokens` |
| Working time (all agents) | `+1 XP per 10 s working`, capped at 60 XP per quest from time (the cap hits at 10 min, so the 20 min halving rule below never triggers for time XP) |
| Quest complete | `+25` |
| Boss victory | `+40`, first-try bonus `+30` |
| Boss defeated after ≥ 2 failures ("Persistence") | `+20` extra |
| Rest survived (compaction) | `+10` |

Anti-farm:
- Time XP rate halves after 20 min of continuous working in one quest, and stops at the per-quest cap.
- Token XP per quest is capped at 400; beyond that, quests still give completion/boss XP.
- Claude Code repeats one message's `usage` on every streamed line of that message, so output tokens are counted once per `message.id`; context XP counts only growth since the last report. Lines read as history (stage start) never earn XP.
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

Each level-up adds 1 bonus point to the hero's current top stat (ties → the first in STR, INT, DEX, VIT, LUK order). Lifetime counters (quests, chapters, edits, reads, shells, rests, first tries, bosses won/lost, bonus points) are saved in `party.json` next to level and XP; old files without them still load.

### 7.5 Loot (M11)
- Every boss victory rolls a drop (`game/loot.rs`), seeded by hero name + win count so the same win always drops the same item. A hero's first win always drops; after that 1 in 4 (little boss), 1 in 3 (normal), 1 in 2 (big).
- Rarity: epic 3% / 6% / 12% by boss size, rare 25%, else common. Bonus: common +1, rare +2–3, epic +4–6.
- Slots: weapon (+STR), armor (+VIT), charm (+INT, DEX or LUK). Name = material (by rarity) + piece + the boss's suffix, e.g. "Steel Sword of Green Tests".
- All drops are kept in `party.json` (`loot`); the best per slot (rarity, then bonus) is equipped and adds to its stat. A drop shows "<hero> found <item>!" in the headline banner for 3 s and is logged; the character sheet lists equipped gear.

### 7.6 Badges (M12)
- 16 badges in `game/badges.rs`, each a check over the hero's lifetime counters and level: First Blood, Flawless, Never Give Up (a win after 3+ fails), On a Roll (5-win streak), Boss Hunter (10 wins), Legend (100), Treasure Hunter (an epic item), Shadow Lord (10 Summons), Bookworm (100 reads), Blacksmith (100 edits), Shell Shocked (100 shell commands), Well Rested (5 rests), Adventurer (50 quests), Saga (chapter 10), Veteran (level 10), Force Pusher.
- Checked every tick; unlocked ids are saved in `party.json` (`badges`), next to the new counters `comebacks` and `force_pushes`. An unlock shows "🏆 <hero>: <badge>!" in the headline banner; several at once (a veteran's first load) show "🏆 <hero> earned N badges!".
- Headline banners queue: one shows for 3 s, then the next. The banner is drawn last, over the scene.
- Character sheet: badge count, unlocked names, and the next one to earn with its hint.

### 7.7 Team fights and daily quests (M13)
- While a boss is up, another hero running a command of the same boss kind **joins the fight** ("joins the fight!" float). The boss lists its fighters.
- A team win pays every fighter the victory XP × 1.5, counts as a win for each (streaks too), plays an attack on each, and shows "⚔ Team attack! A + B". Loot and first-try credit go to the hero whose command won.
- Daily quest (`game/daily.rs`): one per hero per UTC day, rolled from the hero name and the date: win 3 boss fights, finish 3 quests, raise 2 shadows, make 20 edits, read or search 30 times, or run 15 shell commands. Progress counts from the counter value at the start of the day; completing it pays +50 XP once and shows "📜 <hero> — daily quest done". Saved in `party.json` (`daily`); shown on the character sheet. The day turns over at UTC midnight (no timezone dependency).

### 7.8 Pets and skins (M14)
- Pets: config `[pets]` maps an agent kind (or `default`) to cat, dog, dragon or owl; none by default. The pet stands at its hero's front foot in the full layout, follows lunges and hops, walks and sits with it, and grows up at Lv 10 (baby → grown, `tools/gen_pets.py`). Pets are animated: idle (breathing, blinking, tail), a 4-frame walk, sitting, and a trick — the dragon breathes fire, the cat stretches, the dog wags and barks, the owl flaps and tilts its head. The trick plays while the hero attacks, casts or celebrates, and for 0.6 s every 7 s on its own (offset per hero). A party with pets spreads out (hero slots 23 cells instead of 17). Compact and mini layouts show no pets.
- Skins: recolors of the hero sprite unlocked by level: Verdant (Lv 5), Crimson (Lv 10), Royal Gold (Lv 20); the best unlocked one is worn. `skins = false` turns them off. The character sheet names the pet and skin.

### 7.9 Weekly recap and big hero mode (M15)
- `w`: the last 7 days of the quest log (UTC days; each log line's `ended_unix_ms`): quests, XP, bosses won/lost, best run of wins, MVP hero by XP, busiest repo, and quests per day as bars (today last).
- Big hero mode: when the full-layout scene is ≥ 30 rows tall and the party still fits at half width, the scene is drawn at half size and every pixel doubled (heroes, pets, Summons, bosses, props). Text (floats, badges, quips) is redrawn once at normal size; the headline banner is drawn on top as usual.

### 7.10 Party crest (M16)
- Party level = the mean hero level counting XP into the next level (9.4 → Lv 9, bar 40%). The top bar shows "★ Lv N <rank>" and a 6-cell bar; narrow panes drop the rank, then the bar. Ranks: Rookies (≤2), Adventurers (3–5), Guild Heroes (6–9), Veterans (10–14), Champions (15–19), Legends (20–29), Mythic (30+), each with its own color.
- A party level-up with the same heroes shows "🎉 Party Lv N — <rank>!" and 3 s of fireworks (three rockets, bursts with trails). A hero joining or leaving changes the mean but never celebrates.

### 7.11 Settings and first run (M18)
- `s` opens settings for the selected hero (Tab picks the next hero): class and pet for its agent kind, skins, memes, reduced motion. ↑/↓ choose a row, ←/→ (or Enter) change it; the change applies at once (every hero of that kind) and is written to config.toml by a line-based edit that keeps comments and other keys. New panes of that kind get the new class.
- First run: when every hero is Lv 1 with no XP and no quests, a welcome card (what the game is, the keys, `s`) sits in the middle of the scene until any key.

### 7.4 Stamina (context window) — "what happens when context is full"
- `STA% = 100 - context_pct`, where `context_pct = current_context_tokens / context_window_size`.
- `context_window_size`: config map by model id, default 200,000; if usage exceeds the configured size, grow the assumed size to the next known tier (200k → 1M) instead of showing > 100%.
- Thresholds: ≥ 70% used → Tired (a sweat drop every 3 s); ≥ 90% → Exhausted (walk at half speed, a drop every 1.2 s, one toast per quest — sent only when `notify_on_blocked` is on).
- Rest scene: 4 s seated, sky dims, memory particles drift into a journal; then Well Rested sparkles for 2 min. +10 XP, +1 VIT, STA from `compactMetadata.postTokens`.
- New Run: "<hero> — Chapter N · New Run" banner for 3 s; the open quest completes; level, XP and stats stay.
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
- As built (M10): every boss result flashes the boss white for one frame and shakes it ±1 col for 150 ms (the boss, not the whole screen — heroes keep their own motion). Floating text (damage, XP, "learned: …") uses terminal text rising 2 rows instead of a pixel digit font. Screen switches fade in from `k` over 250 ms (`tachyonfx::fx::fade_from`); the ask box sweeps in left-to-right over 350 ms (`fx::sweep_in`). No squash/stretch (too fine for halfblock).
- Body motion (`render/motion.rs`, issue #9): attacks dash up to 5 cols toward the enemy and drift back; held-prop actions step 1 col; thrown spells recoil 1 col; running heroes sway ±1 col, hop 3 px every 6–9 s (per-hero phase, not while Exhausted) and kick up dust at each stride. All of it only while the hero is really working.
- Respect `config.reduced_motion = true`: no shake, no flashes, no dashes/hops/sway/dust, transitions become instant.

### 8.5 Frame loop and performance budget
- Adaptive FPS: 12–15 during actions/battles, 4 when all heroes idle, **0 when the stage pane is not visible** (other tab/workspace focused, or zoomed away).
- Only redraw on change (ratatui diffing; skip frame if world unchanged).
- Budget (Apple Silicon / modern Linux, release build):
  - CPU: < 1% idle, < 3% in battle, measured over 60 s.
  - RSS: < 30 MB.
  - Startup to first frame: < 150 ms.
  - Transcript tailing: only appended bytes read; never on the render task.
- Add a `--bench` flag to `stage --demo` that prints average CPU and frame time after 60 s.
- As built: `--bench` runs 60 s, then prints ticks, redraws, average/worst tick time and loop-busy % of one core. Measured 2026-10-16 (`--scenario stress`, release, Windows Terminal + herdr): 0.34% of one core (process CPU), 15.3 MB peak RSS, 0.5 ms avg / 4.9 ms worst tick. Live stage with one Claude agent: 0.7% / 16 MB. Startup to first frame was not measured separately.

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
skins = true                    # costume recolors at Lv 5 / 10 / 20

[classes]
claude = "Knight"
codex = "Mage"
cursor = "Rogue"
opencode = "Ranger"
gemini = "Cleric"
default = "Bard"

[pets]                          # agent kind → cat | dog | dragon | owl (none by default)
claude = "dragon"

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
1. ~~CLI flags for `herdr plugin pane open`~~ Resolved: `--plugin`, `--entrypoint`, `--placement overlay|split|tab|zoomed`, `--workspace`, `--target-pane`, `--direction right|down`, `--cwd`, `--env`, `--focus` / `--no-focus` — **no `--ratio`** (it opened a 50/50 split; resize afterwards with `herdr pane resize` / `layout.set_split_ratio` at M3). `herdr pane split` does have `--ratio` (sizes the new pane).
2. ~~Detect an existing stage pane~~ Resolved: pane info has no plugin id, only the manifest title as `label` (users can rename it). The stage tags its own pane with metadata token `hf_stage=1` (`pane.report_metadata`), which shows in snapshot `panes[].tokens`; autostart/toggle treat a pane with that token **or** the label as the stage.
3. ~~Plugin panes after a herdr server restart~~ Resolved (observed 2026-10-16 after an overnight restart): the pane is restored with its `label` but **as a plain shell** — the stage command is not re-run, and metadata tokens are gone. `autostart` (which runs on startup) and `toggle-stage` now check `pane.process_info`; a stage-looking pane whose foreground process isn't `herdr-fantasy` is closed and replaced. Also seen once: a fresh stage landed at 59/41 instead of 75/25; five retries all gave 74/26.
4. ~~Event stream vs polling~~ Resolved: status-changed subscriptions are per pane; v1 polls `session.snapshot` (§3.2).
5. Kitty graphics probe results inside herdr panes, locally and over SSH.
6. ~~Transcript field names~~ Resolved in §3.3 (subagent tool is `Agent`; non-zero exit sets `is_error`).
7. Herdr agent `agent` field values for each supported agent (for class mapping). Integrations on herdr 0.9.3: pi, omp, claude, codex, copilot, devin, droid, kimi, opencode, kilo, hermes, qodercli, qwen, cursor, mastracode, antigravity-cli, grok, letta.
8. ~~Windows socket~~ Resolved: named pipe (§3.2). Still open: whether file-change notifications on transcripts are reliable on Windows (M4).

### Resolved
- herdr runs natively on Windows (0.9.3); plugin `platforms` enum = `linux | macos | windows`; socket `%APPDATA%\herdr\herdr.sock`.
- `AgentInfo.agent_session` carries the transcript path, reported by herdr's own integration hook.
- Windows: `./target/release/herdr-fantasy` in the manifest runs the `.exe` (verified with `herdr plugin action invoke kantakan.herdr-fantasy.open-ask`, exit 0) — no per-platform manifest entries needed.
- Windows paths herdr injects: config `%APPDATA%\herdr\plugins\config\<plugin-id>`, state `%LOCALAPPDATA%\herdr\plugins\<plugin-id>`.
- `herdr plugin action invoke` takes the fully-qualified id (`<plugin-id>.<action-id>`); `herdr plugin log list` shows exit code and stderr.
