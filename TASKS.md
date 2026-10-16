# herdr fantasy — Milestones & Tasks

Work top to bottom. Each milestone ends in something runnable. Do not start a milestone
until the previous one's acceptance checks pass. Read `CONTEXT.md`, `SPEC.md`, and `ART.md` first.

Legend: `[ ]` todo · each task lists **Done when** (acceptance criteria).

---

## M0 — Project skeleton
- [x] Cargo project `herdr-fantasy`, edition 2021+, `clap` subcommands from SPEC §4 (stubs that print "not implemented").
  **Done when:** `cargo build` passes, `herdr-fantasy --help` lists all subcommands.
- [x] Dependencies: tokio, serde, serde_json, toml, clap, anyhow, thiserror, tracing, tracing-appender, ratatui, crossterm, image, tachyonfx, directories, sha2, insta (dev), rand + rand_chacha (seeded). Also tracing-subscriber (needed for file logging); `image` with PNG only.
- [x] `herdr-plugin.toml` exactly as SPEC §4.3 (platforms linux/macos/windows). Resolved: the `./target/release/…` path works on Windows.
  **Done when:** `herdr plugin link .` succeeds with no warnings on Windows and one Unix platform. (Windows ✓ 2026-10-16; Unix link still to check on a Mac/Linux box.)
- [x] `config.rs` with defaults from SPEC §9, per-key fallback on invalid values.
  **Done when:** unit tests cover missing file, partial file, invalid values.
- [x] Logging to `$HERDR_PLUGIN_STATE_DIR/logs/` (fallback `~/.local/state/herdr-fantasy/logs/`).
- [x] CI (GitHub Actions): fmt, clippy `-D warnings`, test on macOS + Linux + Windows.

## M1 — Demo mode + halfblock renderer (no herdr needed)
- [x] Placeholder pipeline: `assets/placeholders/*.txt` → PNG + Aseprite-style JSON (ART.md §8), via `build.rs` or an `xtask`.
  **Done when:** slime + 6 class placeholders + 1 boss + Rubber Duck load at runtime.
- [x] `render/halfblock.rs`: draw a sprite frame with transparency onto a ratatui buffer.
- [x] `game/world.rs` + `game/hero.rs` state machine for: idle/sit, walk, attack, cast, open, raise_hand, victory.
- [x] `game/demo.rs`: `party` scenario with seeded RNG and `--speed`.
- [x] `render/layout.rs`: full / compact / mini layouts by pane size (SPEC §5.2–5.3; thresholds now derived from sprite sizes).
- [x] Footer rows with XP and stamina bars.
- [x] Adaptive FPS loop (SPEC §8.5) and "skip frame if unchanged".
  **Done when:** `herdr-fantasy stage --demo` runs in any terminal (including Windows Terminal), shows 3 heroes doing the scripted party scenario, resizing switches layouts without crashing, CPU < 3% during battle. (Measured 2026-10-16, release, Windows Terminal + herdr, 30 s of the party loop: 0.8% of one core, 21 MB RSS.)
- [x] Snapshot tests: render ticks 0, 50, 200 of `party --seed 1` with `TestBackend` + insta.

## M2 — Live herdr connection (status-level game)
- [x] `herdr/client.rs`: connect to socket, request/response by id, subscription stream. Works on Windows AF_UNIX (resolve SPEC §15 Q9).
- [x] `herdr/types.rs`: AgentInfo, PaneInfo, events; ignore unknown fields. Fixtures in `tests/fixtures/` captured from a real herdr (scrubbed of real paths/titles).
- [x] `herdr/cache.rs`: poll `session.snapshot` (SPEC §3.2 — per-pane status subscriptions made events impractical), send only changes, reconnect with backoff + banner.
- [x] Map herdr status → hero state (idle, working auto-battle, blocked ❗, done victory, gone walks off).
- [x] Party = agents in the stage's own workspace (`HERDR_WORKSPACE_ID`); `show_all_workspaces` option.
- [x] Quest title from `terminal_title_stripped`; quest lifecycle (SPEC §6.4).
- [x] `game/repo.rs` repo identity (SPEC §6.1) from agent `foreground_cwd`/`cwd`.
- [x] Hero identity + slots + class mapping (SPEC §6.2).
- [x] Time-based XP for all agents, levels, level-up effect (SPEC §7).
- [x] `game/persist.rs` save/load party per repo, atomic writes, corrupt-file handling.
- [x] Visibility gating: 0 fps when the stage pane's tab/workspace is not focused.
- [ ] **Manual check left for KantaKan:** open a Codex pane next to a Claude pane and confirm both heroes appear and track herdr's sidebar.
  **Done when:** inside herdr with real Claude Code + Codex panes, `herdr plugin pane open --plugin kantakan.herdr-fantasy --entrypoint stage` shows both heroes and their states change within 300 ms of herdr's sidebar; killing/restarting the stage keeps levels. (2026-10-16: verified with a real Claude pane via `herdr plugin pane open … --target-pane wC:p1 --direction down --no-focus`; XP saved on Ctrl-C and reloaded. Codex not yet tried.)

## M3 — Auto-open + notifications + ask popup
- [x] `autostart`: open stage per SPEC §5.1 without stealing focus, never duplicate. Q1–Q2 resolved in SPEC §15; Q3 (server restart) still open.
- [x] `toggle-stage` action.
- [x] `notification.show` once per blocked transition (respect config).
- [x] `ask` popup: JRPG box, portrait talk animation, typewriter text from `pane.read --source detection`, options Go/Next/Close, `agent.focus` on Go. Never sends keys to agents.
- [x] `open-ask` action opens the popup for the most recently blocked hero.
- [x] README section: keybinding snippet `prefix+a` → `kantakan.herdr-fantasy.open-ask`.
- [ ] **Manual check left for KantaKan:** restart herdr (stage should appear in every workspace exactly once), let an agent block (❗ + one toast), press `prefix+a` (after adding the README snippet), `Go` focuses it, `Esc` closes.
  **Done when:** opening herdr shows the stage automatically in every workspace; a blocked agent shows ❗ + one toast; `prefix+a` opens the dialogue and Go focuses that pane; `Esc` always closes.

## M4 — Claude transcript reader (rich events) — ADR-0001
- [x] Capture a real Claude transcript into `tests/fixtures/` (reads, edits, Bash + PowerShell pass/fail, a subagent, a compaction — scrubbed, from real transcripts). Resolve SPEC §15 Q6 and update SPEC §3.3.
- [x] `transcript/tail.rs`: follow by byte offset, partial lines, truncation/replacement, no replay on first sight (SPEC §11).
- [x] `transcript/claude.rs`: entries → agent-neutral `GameEvent` (pure, fixture-tested).
- [x] Wire `agent_session.value` from herdr cache → one tailer per live Claude Agent; path change = new Chapter.
- [x] Tool → animation mapping (SPEC §5.4), including per-command moves (grep reads a book, git push sends a pigeon, …) and the `Skill` spellbook; action queue with combo collapse.
- [x] Quest title from the transcript user prompt; `store_prompt_text` option.
  **Done when:** in a Claude Code pane, reading files opens chests and edits swing swords with line-count damage within 1 s (**measured ~2.3 s**: Claude Code's transcript flush, see SPEC §3.3 — accepted, ADR-0001); restarting the stage doesn't replay old actions; `/clear` starts a new Chapter. (Verified live: `rg` → "reading the big book" in the real stage. `/clear` covered by `tails_send_history_then_new_lines_then_new_chapter`; not tried live because it would wipe this session.)

## M5 — Bosses
- [x] Boss roster, species pick, and size tiers in `game/boss.rs` (SPEC §6.3) — landed early, with 27 placeholder sprites.
- [x] `game/boss.rs` classifier (SPEC §6.3) with user rules from config. Unit tests for every built-in pattern.
- [x] Boss enter/battle/victory/laugh flow, cosmetic HP, first-try + persistence bonuses.
- [x] Verify that `is_error` marks failed Bash and PowerShell runs; update SPEC §6.3. (Yes — but a `| tail` pipe hides it, so failure markers in the output count too.)
  **Done when:** `cargo test` failing then passing in a Claude session shows Test Golem laughing, losing 25% HP, then dying with the right XP. (2026-10-16, live in this repo: `cargo test --no-such-flag` → Snapshot Mimic at 75%; `cargo test game::boss` → defeated, +40 XP. The species is per command, so `cargo test` here summons the Mimic, not the Golem.)

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
