# Read agent Transcripts instead of running our own Claude Code hook bridge

herdr's built-in agent integrations already report each session (`AgentInfo.agent_session`: the transcript path, or — what Claude panes actually report on herdr 0.9.3 — the session id, from which the path is derived as `~/.claude/projects/<slug(cwd)>/<id>.jsonl`), and the transcript records every tool call, tool result (`is_error`), and token usage. So the Stage tails the Transcript as its single source of rich events, and we ship no `hook` / `install-hooks` subcommands, no IPC socket, and no `hf_*` pane tokens. That removes editing the user's `settings.json`, a cross-platform IPC layer (named pipes on Windows), and a strict latency budget for the hook process.

## Consequences

- Animations lag the live agent by one transcript flush — measured ~2.3 s (Claude Code batches its writes). Good enough for an ambient pane; a short tool may animate after it already finished.
- Permission-prompt dialogue text comes from `pane.read`, not from the `Notification` hook message.
- Our state isn't shown in herdr's sidebar.
- Revisit hooks only if measured transcript lag is noticeably bad.
