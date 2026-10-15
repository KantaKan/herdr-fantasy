# Working rules for coding agents on herdr fantasy

- Read `CONTEXT.md`, `SPEC.md`, `ART.md`, and `TASKS.md` before writing code (and `docs/adr/` for the area you touch). `TASKS.md` is the order of work.
- Do one milestone at a time. Finish its "Done when" checks before moving on, then tick the boxes.
- When SPEC marks something with ⚠️ or lists it in §15 "Open questions", verify it against the real tool (`herdr api schema --json`, a real transcript, a real terminal) and **update SPEC.md** with what you found. Never guess silently.
- Performance budget in SPEC §8.5 is a requirement, not a goal. Measure with `stage --demo --bench`.
- Transcript reading is read-only, tail-only, and never runs on the render task. We install no hooks of our own (ADR-0001).
- No network access, no telemetry, no vendor logos or mascots, no copyrighted characters. All art uses the Sweetie 16 palette in ART.md.
- Keep game rules (XP, bosses, states) in pure functions with unit tests. Keep I/O at the edges.
- `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` must pass before you say a task is done.
- Windows is a first-class platform: no Unix-only APIs without a Windows path, and test on Windows Terminal.

## Agent skills

### Issue tracker

GitHub Issues on `KantaKan/herdr-fantasy` via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default five labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: root `CONTEXT.md` + `docs/adr/`. See `docs/agents/domain.md`.
