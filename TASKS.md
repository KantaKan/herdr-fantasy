# herdr fantasy — Milestones & Tasks

Work top to bottom. Each milestone ends in something runnable. Do not start a milestone
until the previous one's acceptance checks pass. Read `CONTEXT.md`, `SPEC.md`, and `ART.md` first.

Legend: `[ ]` todo · each task lists **Done when** (acceptance criteria).

---

## M0 — Project skeleton
- [ ] Cargo project `herdr-fantasy`, edition 2021+, `clap` subcommands from SPEC §4 (stubs that print "not implemented").
  **Done when:** `cargo build` passes, `herdr-fantasy --help` lists all subcommands.
- [ ] Dependencies: tokio, serde, serde_json, toml, clap, anyhow, thiserror, tracing, tracing-appender, ratatui, crossterm, image, tachyonfx, directories, sha2, insta (dev), rand + rand_chacha (seeded).
- [ ] `herdr-plugin.toml` exactly as SPEC §4.3 (platforms linux/macos/windows). Resolve SPEC §15 Q8 (`.exe` path on Windows).
  **Done when:** `herdr plugin link .` succeeds with no warnings on Windows and one Unix platform.
- [ ] `config.rs` with defaults from SPEC §9, per-key fallback on invalid values.
  **Done when:** unit tests cover missing file, partial file, invalid values.
- [ ] Logging to `$HERDR_PLUGIN_STATE_DIR/logs/` (fallback `~/.local/state/herdr-fantasy/logs/`).
- [ ] CI (GitHub Actions): fmt, clippy `-D warnings`, test on macOS + Linux + Windows.

## M1 — Demo mode + halfblock renderer (no herdr needed)
- [ ] Placeholder pipeline: `assets/placeholders/*.txt` → PNG + Aseprite-style JSON (ART.md §8), via `build.rs` or an `xtask`.
  **Done when:** slime + 6 class placeholders + 1 boss + Rubber Duck load at runtime.
- [ ] `render/halfblock.rs`: draw a sprite frame with transparency onto a ratatui buffer.
- [ ] `game/world.rs` + `game/hero.rs` state machine for: idle/sit, walk, attack, cast, open, raise_hand, victory.
- [ ] `game/demo.rs`: `party` scenario with seeded RNG and `--speed`.
- [ ] `render/layout.rs`: full / compact / mini layouts by pane size (SPEC §5.2–5.3).
- [ ] Footer rows with XP and stamina bars.
- [ ] Adaptive FPS loop (SPEC §8.5) and "skip frame if unchanged".
  **Done when:** `herdr-fantasy stage --demo` runs in any terminal (including Windows Terminal), shows 3 heroes doing the scripted party scenario, resizing switches layouts without crashing, CPU < 3% during battle.
- [ ] Snapshot tests: render ticks 0, 50, 200 of `party --seed 1` with `TestBackend` + insta.

## M2 — Live herdr connection (status-level game)
- [ ] `herdr/client.rs`: connect to socket, request/response by id, subscription stream. Works on Windows AF_UNIX (resolve SPEC §15 Q9).
- [ ] `herdr/types.rs`: AgentInfo, PaneInfo, events; ignore unknown fields. Fixtures in `tests/fixtures/` captured from a real herdr.
- [ ] `herdr/cache.rs`: subscribe to SPEC §3.2 events, debounced `agent.list` refetch, `events_lost` recovery, reconnect with backoff + banner.
- [ ] Map herdr status → hero state (idle, working auto-battle, blocked ❗, done victory, gone walks off).
- [ ] Party = agents in the stage's own workspace (`HERDR_WORKSPACE_ID`); `show_all_workspaces` option.
- [ ] Quest title from `terminal_title_stripped`; quest lifecycle (SPEC §6.4).
- [ ] `game/repo.rs` repo identity (SPEC §6.1) from agent `foreground_cwd`/`cwd`.
- [ ] Hero identity + slots + class mapping (SPEC §6.2).
- [ ] Time-based XP for all agents, levels, level-up effect (SPEC §7).
- [ ] `game/persist.rs` save/load party per repo, atomic writes, corrupt-file handling.
- [ ] Visibility gating: 0 fps when the stage pane's tab/workspace is not focused.
  **Done when:** inside herdr with real Claude Code + Codex panes, `herdr plugin pane open --plugin kantakan.herdr-fantasy --entrypoint stage` shows both heroes and their states change within 300 ms of herdr's sidebar; killing/restarting the stage keeps levels.

## M3 — Auto-open + notifications + ask popup
- [ ] `autostart`: open stage per SPEC §5.1 without stealing focus, never duplicate. Resolve SPEC §15 Q1–Q3 and update SPEC.
- [ ] `toggle-stage` action.
- [ ] `notification.show` once per blocked transition (respect config).
- [ ] `ask` popup: JRPG box, portrait talk animation, typewriter text from `pane.read --source detection`, options Go/Next/Close, `agent.focus` on Go. Never sends keys to agents.
- [ ] `open-ask` action opens the popup for the most recently blocked hero.
- [ ] README section: keybinding snippet `prefix+a` → `kantakan.herdr-fantasy.open-ask`.
  **Done when:** opening herdr shows the stage automatically in every workspace; a blocked agent shows ❗ + one toast; `prefix+a` opens the dialogue and Go focuses that pane; `Esc` always closes.

## M4 — Claude transcript reader (rich events) — ADR-0001
- [ ] Capture a real Claude transcript into `tests/fixtures/` (reads, edits, Bash + PowerShell pass/fail, a subagent, a compaction). Resolve SPEC §15 Q6 and update SPEC §3.3.
- [ ] `transcript/tail.rs`: follow by byte offset, partial lines, truncation/replacement, no replay on first sight (SPEC §11).
- [ ] `transcript/claude.rs`: entries → agent-neutral `GameEvent` (pure, fixture-tested).
- [ ] Wire `agent_session.value` from herdr cache → one tailer per live Claude Agent; path change = new Chapter.
- [ ] Tool → animation mapping (SPEC §5.4), action queue with combo collapse.
- [ ] Quest title from the transcript user prompt; `store_prompt_text` option.
  **Done when:** in a Claude Code pane, reading files opens chests and edits swing swords with line-count damage within 1 s; restarting the stage doesn't replay old actions; `/clear` starts a new Chapter.

## M5 — Bosses
- [ ] `game/boss.rs` classifier (SPEC §6.3) with user rules from config. Unit tests for every built-in pattern.
- [ ] Boss enter/battle/victory/laugh flow, cosmetic HP, first-try + persistence bonuses.
- [ ] Verify that `is_error` marks failed Bash and PowerShell runs; update SPEC §6.3.
  **Done when:** `cargo test` failing then passing in a Claude session shows Test Golem laughing, losing 25% HP, then dying with the right XP.

## M6 — Stamina, rest, new runs
- [ ] Stamina from the latest transcript usage entry (SPEC §7.4).
- [ ] Stamina bar, Tired/Exhausted states, one toast per quest.
- [ ] Rest scene + Well Rested buff on the transcript compaction marker or `agent_session` source `compact`.
- [ ] New Run banner when a Chapter starts; level kept.
- [ ] Token-based XP with caps (SPEC §7.1); stats STR/INT/DEX/VIT/LUK.
  **Done when:** running `/compact` in Claude plays the rest scene and refills STA; `/clear` starts a new chapter without losing level.

## M7 — Quest log, character sheet, town
- [ ] Quest log screen `q` (SPEC §5.6), reading `quests.jsonl`.
- [ ] Character sheet `c` (SPEC §5.8).
- [ ] Town `t` (SPEC §5.7): building ranking, tiers, theme by language, commit-count decorations, heroes placed by last touched file from the transcript. Git calls cached, never on the render thread.
  **Done when:** all three screens work in full and compact layouts and survive restarts.

## M8 — Summons, Joke Classes, Rubber Duck, Quips
- [ ] `game/summon.rs`: subagent `tool_use` → Summon, `tool_result` → dismiss; XP to summoning Hero; "Shadows raised" counter. Unit tests.
- [ ] Joke Class roll seeded by `tool_use` id; 12 placeholder sprites (ART.md §4b); shadow rise/sink animation.
- [ ] Draw up to 3 Summons per Hero, `+N` beyond.
- [ ] `game/duck.rs`: triggers + 2-min rate limit (SPEC §6.6), pure + unit tests. Duck sprite + speech bubble.
- [ ] Quip table keyed by event; `memes` config toggle.
  **Done when:** a Claude run with 2 parallel subagents shows two Joke Class Summons rising and sinking, a twice-failed `cargo test` makes the Duck quip, and `memes = false` silences every Quip.

## M9 — Kitty graphics (Mac/Linux upgrade; Windows Terminal stays halfblock)
- [ ] `render/backend.rs` probe + `render` config override + `doctor` output.
- [ ] `render/kitty.rs`: transmit frames once by image id, animate via placements, clean up on hide/resize/exit.
- [ ] Verify in Ghostty inside herdr locally and over `herdr --remote` (SPEC §15 Q5); halfblock fallback when the probe fails.
  **Done when:** Ghostty shows crisp sprites, a non-Kitty terminal shows halfblock, and CPU budget still holds.

## M10 — Polish, perf, release
- [ ] Effects pass: hit flash, floating numbers, shake, tachyonfx transitions, `reduced_motion`.
- [ ] `stage --demo --scenario stress --bench` meets SPEC §8.5 budget; record numbers in README.
- [ ] Real sprites replace placeholders (art can land any time; pipeline unchanged).
- [ ] README: GIF from `--demo --seed`, install (`herdr plugin install KantaKan/herdr-fantasy`; needs a Rust toolchain), keybinding snippet, herdr's Claude integration requirement, privacy note (all local, no network, transcripts read-only).
- [ ] GitHub topic `herdr-plugin`; tag `v0.1.0`.

---

## Manual QA checklist (run before each release)
- [ ] Windows Terminal + native herdr: halfblock, auto-open, ask popup, transcript events.
- [ ] Ghostty + herdr local: Kitty sprites, auto-open, ask popup, transcript events.
- [ ] A terminal without Kitty graphics: halfblock everywhere.
- [ ] `herdr --remote` over SSH: stage runs on the server, renders on the client.
- [ ] 1, 3, and 8 agents; tiny pane (mini mode); very wide pane.
- [ ] Kill the herdr server mid-battle; stage reconnects; no state loss.
- [ ] Claude with a subagent storm (10 parallel): Summons cap at 3 + `+N`, CPU budget holds.
