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
- [x] Codex pane next to a Claude pane: both heroes appear (Mage + Claude's class) and follow herdr's status. (Verified live 2026-10-16 in a test workspace.)
  **Done when:** inside herdr with real Claude Code + Codex panes, `herdr plugin pane open --plugin kantakan.herdr-fantasy --entrypoint stage` shows both heroes and their states change within 300 ms of herdr's sidebar; killing/restarting the stage keeps levels. (2026-10-16: verified with a real Claude pane via `herdr plugin pane open … --target-pane wC:p1 --direction down --no-focus`; XP saved on Ctrl-C and reloaded. Codex not yet tried.)

## M3 — Auto-open + notifications + ask popup
- [x] `autostart`: open stage per SPEC §5.1 without stealing focus, never duplicate. Q1–Q2 resolved in SPEC §15; Q3 (server restart) still open.
- [x] `toggle-stage` action.
- [x] `notification.show` once per blocked transition (respect config).
- [x] `ask` popup: JRPG box, portrait talk animation, typewriter text from `pane.read --source detection`, options Go/Next/Close, `agent.focus` on Go. Never sends keys to agents.
- [x] `open-ask` action opens the popup for the most recently blocked hero.
- [x] README section: keybinding snippet `prefix+a` → `kantakan.herdr-fantasy.open-ask`.
- [x] Agent blocks on a permission prompt → `❗ needs you`; the ask popup shows the hero portrait and the real question, `Go` focuses the agent, `Esc` closes; a new workspace gets exactly one stage from `workspace.created`. (Verified live 2026-10-16.) herdr itself reports `done`, not `blocked`, for an `AskUserQuestion` prompt, and flickers to `working` while a permission prompt is still open — herdr's detection, not ours.
- [x] Restart herdr: autostart puts exactly one stage in each of 4 workspaces, all on the new binary (verified 2026-10-16 after KantaKan's `herdr server stop`).
- [x] One "needs you" toast when an agent blocks (confirmed by KantaKan 2026-10-16).
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
- [x] Stamina from the latest transcript usage entry (SPEC §7.4).
- [x] Stamina bar, Tired/Exhausted states, one toast per quest.
- [x] Rest scene + Well Rested buff on the transcript compaction marker or `agent_session` source `compact`.
- [x] New Run banner when a Chapter starts; level kept.
- [x] Token-based XP with caps (SPEC §7.1); stats STR/INT/DEX/VIT/LUK.
- [x] `/compact` and `/clear` in a real Claude pane (verified live 2026-10-16): compaction shows the Rest scene and stamina refills (69% → 98%); `/clear` shows `Chapter 2 · New Run` and keeps the level. Found and fixed: the typed `/compact` line had become a quest title.
  **Done when:** running `/compact` in Claude plays the rest scene and refills STA; `/clear` starts a new chapter without losing level.

## M7 — Quest log, character sheet, town
- [x] Quest log screen `q` (SPEC §5.6), reading `quests.jsonl`.
- [x] Character sheet `c` (SPEC §5.8).
- [x] Town `t` (SPEC §5.7): building ranking, tiers, theme by language, commit-count decorations, heroes placed by last touched file from the transcript. Git calls cached, never on the render thread.
  **Done when:** all three screens work in full and compact layouts and survive restarts. (2026-10-16: full layout checked in previews; compact checked live in the real stage — Forge Village from this repo's git, character sheet Lv4, quest log with 12 quests loaded from disk after a stage restart.)

## M8 — Summons, Joke Classes, Rubber Duck, Quips
- [x] `game/summon.rs`: subagent `tool_use` → Summon, `tool_result` → dismiss; XP to summoning Hero; "Shadows raised" counter. Unit tests.
- [x] Joke Class roll seeded by `tool_use` id; 12 placeholder sprites (ART.md §4b); shadow rise/sink animation.
- [x] Draw up to 3 Summons per Hero, `+N` beyond.
- [x] `game/duck.rs`: triggers + 2-min rate limit (SPEC §6.6), pure + unit tests. Duck sprite + speech bubble.
- [x] Quip table keyed by event; `memes` config toggle.
- [x] 2 real parallel subagents (verified live 2026-10-16). Found and fixed: Claude Code now runs subagents async — the `Agent` result returns in ~2 s, so Summons sank at once; they now stay until the `<agent-message>` hand-back (`async_subagents_stay_until_they_hand_back`).
  **Done when:** a Claude run with 2 parallel subagents shows two Joke Class Summons rising and sinking, a twice-failed `cargo test` makes the Duck quip, and `memes = false` silences every Quip. (Duck verified live 2026-10-16: two failing `cargo test` runs → log `duck quips (StubbornBoss): Quack. Explain it to me, line by line.`)

## M9 — Kitty graphics (Mac/Linux upgrade; Windows Terminal stays halfblock)
- [ ] `render/backend.rs` probe + `render` config override + `doctor` output.
- [ ] `render/kitty.rs`: transmit frames once by image id, animate via placements, clean up on hide/resize/exit.
- [ ] Verify in Ghostty inside herdr locally and over `herdr --remote` (SPEC §15 Q5); halfblock fallback when the probe fails.
  **Done when:** Ghostty shows crisp sprites, a non-Kitty terminal shows halfblock, and CPU budget still holds.

## M10 — Polish, perf, release
- [x] Effects pass: hit flash, floating numbers, shake, tachyonfx transitions, `reduced_motion`.
- [x] `stage --demo --scenario stress --bench` meets SPEC §8.5 budget; record numbers in README. (0.34% CPU, 15.3 MB.)
- [ ] Real sprites replace placeholders (art can land any time; pipeline unchanged). (Placeholders are generated by `tools/gen_*.py`; real Aseprite exports drop into `assets/sprites/`.)
- [x] README: GIF from `--demo --seed`, install (`herdr plugin install KantaKan/herdr-fantasy`; needs a Rust toolchain), keybinding snippet, herdr's Claude integration requirement, privacy note (all local, no network, transcripts read-only).
- [x] GitHub topic `herdr-plugin`; tag `v0.1.0`. (Published 2026-10-16: https://github.com/KantaKan/herdr-fantasy/releases/tag/v0.1.0)

---

## M11 — Loot
- [x] `game/loot.rs` (pure): a boss victory may drop an item (always on a hero's first win, else ~1 in 3, better odds for bigger bosses). Name = material + piece + boss-themed suffix ("Sword of Green Tests", "Cursed Cape of Merge Conflicts"), rarity common/rare/epic, one stat bonus. Seeded by hero + win count, so the same win always drops the same thing.
- [x] Saved in the hero's progress; the best item per slot (weapon, armor, charm) is equipped and adds to its stat.
- [x] Float + log line on a drop; the character sheet lists the equipped gear.
  **Done when:** beating bosses drops items that survive a restart and raise the stats on the character sheet. (2026-10-16: `first_boss_win_drops_loot_that_raises_a_stat`, the persist round trip, and the demo — "Claude found Rusty Shield of the Early Sahur!", Gear on the sheet. The drop shows in the headline banner rather than a float, so it doesn't fight the +XP float.)

## M12 — Achievements
- [x] `game/badges.rs` (pure): a table of badges, each a check over the hero's lifetime counters (First Try, Persistent, Force Pusher, 10/100 Bosses, Shadow Lord, Bookworm, Blacksmith, Well Rested, Comeback, Chapter 10, Level 10 …).
- [x] Unlocks checked after each event; a "🏆 <badge>" banner + float; saved with progress; badges section on the character sheet.
  **Done when:** reaching a badge's condition shows the banner once, and it stays unlocked after a restart. (2026-10-16: `badges_unlock_once_and_queue_behind_the_loot_banner`; demo shows the loot banner, then "🏆 Claude: First Blood!". Badges are saved with the rest of `Lifetime`, covered by the persist round trip. Night Owl was dropped: no local-time source without a new dependency.)

## M13 — Party combos + daily quest
- [x] A second hero running a boss command while a boss is up **joins the fight**: team attack animation, both get the victory XP plus a combo bonus.
- [x] Daily quest per hero, rolled from the UTC date ("Win 5 boss fights", "Finish 3 quests", "Raise 2 shadows" …): progress on the character sheet, bonus XP when done.
  **Done when:** two heroes beating one boss both get XP with the combo float; a daily quest completes and pays once per day. (2026-10-16: `a_second_hero_joins_the_fight_and_both_get_team_xp`, `daily_quest_pays_once`, `completes_once_and_rolls_over_each_day`.)

## M14 — Pets + hero skins
- [x] Pets (`[pets] claude = "cat"`; cat, dog, dragon, owl): a small sprite that follows its hero, hops when it hops, sits at camp; grows up at Lv 10 (two stages instead of the planned three, to keep the art small).
- [x] Skins: costume recolors unlocked at Lv 5 / 10 / 20; the best unlocked is worn (`skins = false` turns them off — simpler than a per-hero `skin` choice).
  **Done when:** a configured pet follows its hero in the stage and the GIF; a Lv 10 hero shows its unlocked skin. (2026-10-16: demo party — Lv 12 Crimson Claude with a grown dragon, Lv 6 Verdant Mage with a baby owl, a cat — in previews and snapshots; GIF refresh at the end of M15.)

## M15 — Weekly recap + big hero mode
- [x] `w` opens a weekly recap from the quest log: quests, XP, bosses won/lost, best streak, busiest repo, per-day bars.
- [x] Tall stage panes (≥ 30 rows) draw heroes, bosses and Summons at 2× size.
  **Done when:** the recap matches the quest log, and a tall pane shows big sprites without breaking the other layouts or the CPU budget. (2026-10-17: recap checked against the demo quest log — 3 quests, 189 XP, 1 won / 2 lost; big mode in a 200×40 preview; `layouts_by_size` covers when it turns on. CPU: the stress bench measured 3.45% in a herdr pane today vs 3.68% for the pre-M11 build in the same pane — no regression, but this machine was busier than when the 0.34% in the README was measured, so the budget wants a re-check on a quiet machine.)

## M16 — Party crest
- [x] Top bar: "★ Lv 9 Guild Heroes" with a progress bar to the next party level; the party level is the mean hero level including XP into the next level. Ranks: Rookies, Adventurers, Guild Heroes, Veterans, Champions, Legends, Mythic.
- [x] Party level-up: "🎉 Party Lv N — <rank>!" banner and fireworks in the sky; a hero joining or leaving doesn't count.
  **Done when:** the demo party levels up with the banner and fireworks. (2026-10-17: Lv 6 → 7 in the demo; `party_level_up_celebrates_but_a_newcomer_does_not`.)

## M17 — Codex rich events
- [ ] Find and tail Codex session transcripts (`~/.codex/sessions/**.jsonl`); verify the shapes against a real one (SPEC §3.3). (Envelope verified; payload shapes still need a real session with tool calls — the auto-mode safety check blocked running `codex exec --full-auto` from here.)
- [x] Map Codex tool calls (shell, apply_patch, …) to the same Entry kinds: actions, bosses, XP, compaction. (`transcript/codex.rs`; tails pick a parser per agent kind; rollout found by session id and cached.)
  **Done when:** a Codex pane's hero swings, reads, fights bosses and earns XP like a Claude hero.

## M18 — Settings screen + first-run welcome
- [x] `s` opens settings: class per agent, pet, skins, memes, reduced motion; saved to config.toml (comments kept).
- [x] First run (no saved heroes): a welcome card with the keys and a pointer to `s`.
  **Done when:** picking a class or pet in the stage changes the hero without editing files. (2026-10-17: `class_and_pet_cycle_for_the_whole_kind`, `edits_keep_comments_and_other_keys`, `welcome_card_for_a_new_party_until_a_key`; settings screen checked in a preview.)

## M19 — Prebuilt binaries
- [x] Release workflow builds Windows, macOS (arm64 + x86_64) and Linux binaries on each tag.
- [x] Install uses a prebuilt binary when one matches, else builds with cargo.
  **Done when:** a fresh machine without Rust installs the plugin. (2026-10-17: v0.3.0's release workflow attached all 4 binaries + SHA-256 files; `herdr plugin install` on Windows took 7 s and used the downloaded binary — nothing compiled. Linux/macOS install scripts not yet run on a real machine.)

## M20 — Performance pass
- [x] Profile the stress bench; cut per-frame work (redraw only what changed, cache recolors, cheaper idle).
- [x] Budget (< 3% CPU, < 30 MB) re-measured in a real terminal on a quiet machine.
  **Done when:** the stress bench is under budget with numbers in the README. (2026-10-17: profiled with `perf_split` / `perf_changed_cells` (ignored tests): update 7 µs, render 110 µs per frame, ~280 changed cells — the rest is terminal output, which only redraws changed cells already. Real process CPU 0.94%, 24.3 MB. Nothing needed cutting; the earlier 3.45% was wall-clock time on a busy machine.)

## M21 — Dragon Quest battle screen
- [x] Message window (`game/battle.rs`): narrated lines type out, two shown, a short queue; every action, enemy, boss, loot and level-up is a line.
- [x] Field enemies with HP: slime waves (Slime, Red Slime, rare Metal Slime) take edit damage, blink and show the damage, die, and the next wave "draws near".
- [x] Layout: top bar, battlefield (heroes + pets left, enemies right, enemy-name window), and a DQ band — one window per hero (name, Lv, HP, status; selected = yellow border) beside the message window. Footer and quest line go. Full = 14 rows, compact (chibis) = 11.
- [x] Effects: slash marks on hits, enemy blink, a boss-appears transition.
  **Done when:** the demo plays as a narrated DQ-style battle in full and compact panes, with tests and a new GIF. (2026-10-21: `battles_are_narrated_and_edits_hit_the_field_wave`, `dq_band_shows_party_windows_and_messages`, battle.rs tests; previews at 112×18, 90×14, 80×11; GIF refreshed.)

## M22 — Class skills in battle
- [x] Ranged classes (Mage, Ranger, Bard) stay home and shoot: fireballs, arrows and notes fly to the target and hit on arrival. (The Wild Card stays melee — it charges in and slashes with a random card.)
- [x] Each class has its trait: Rogue critical hits, Knight blocks the counterattack, Cleric heals the party, Bard's song raises everyone's attack, Sahur Knight smashes, Wild Card's draw is random.
  **Done when:** every class fights differently in the demo and the messages say so. (2026-10-21: `every_class_fights_its_own_way`; fireball and arrows seen in demo frames.)

## M23 — Monster variety
- [x] New field monsters (bat, skeleton, goblin, mushroom) with their own sprites, names and HP, alongside the slimes.
- [x] Each town theme has its own monster table.
  **Done when:** two themes show different monsters. (2026-10-21: `mobs_take_damage_die_and_waves_vary` checks Forge Village ≠ Lantern City; goblins and red slimes in the demo.)

## M24 — Bosses strike back
- [ ] While a boss fight runs, the boss uses a named move every few seconds ("The Test Golem uses Assertion Slam!"); the party windows shake and the heroes take cosmetic damage.
  **Done when:** a long boss fight shows the boss attacking.

## M25 — Victory screen
- [ ] A boss defeat opens a DQ results window: victorious, EXP, Gold, and loot, for a few seconds.
  **Done when:** the demo shows the results window after the boss falls.

## M26 — Gold, shop and inn
- [ ] Gold from monsters and bosses, saved per hero; shown on the character sheet and the party window row.
- [ ] Town: a shop to buy gear with gold and an inn to buy the Well Rested buff.
  **Done when:** gold earned in fights buys an item in the town shop.

## Manual QA checklist (run before each release)
- [ ] Windows Terminal + native herdr: halfblock, auto-open, ask popup, transcript events.
- [ ] Ghostty + herdr local: Kitty sprites, auto-open, ask popup, transcript events.
- [ ] A terminal without Kitty graphics: halfblock everywhere.
- [ ] `herdr --remote` over SSH: stage runs on the server, renders on the client.
- [x] 1, 3, and 8 agents; tiny pane (mini mode); very wide pane. (`any_size_renders_without_panic`, `layouts_by_size`, the 8-hero `stress` scenario.)
- [ ] Kill the herdr server mid-battle; stage reconnects; no state loss.
- [x] Claude with a subagent storm (10 parallel): Summons cap at 3 + `+N`, CPU budget holds. (`summon_storm_shows_three_and_a_count` — found and fixed the `+N` being hidden behind the next hero; stress bench 1.5% of one core, 15.7 MB.)
