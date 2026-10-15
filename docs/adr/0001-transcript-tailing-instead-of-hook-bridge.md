# Read agent Transcripts instead of running our own Claude Code hook bridge

herdr's built-in agent integrations already report each session's transcript path (`AgentInfo.agent_session`), and the transcript records every tool call, tool result (`is_error`), and token usage. So the Stage tails the Transcript as its single source of rich events, and we ship no `hook` / `install-hooks` subcommands, no IPC socket, and no `hf_*` pane tokens. That removes editing the user's `settings.json`, a cross-platform IPC layer (named pipes on Windows), and a strict latency budget for the hook process.

## Consequences

- Animations can lag the live agent by up to one transcript flush.
- Permission-prompt dialogue text comes from `pane.read`, not from the `Notification` hook message.
- Our state isn't shown in herdr's sidebar.
- Revisit hooks only if measured transcript lag is noticeably bad.
