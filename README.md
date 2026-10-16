# herdr fantasy

Your coding agents as a fantasy RPG party, living in a [herdr](https://herdr.dev) pane.
While agents work they fight slimes and bosses that mirror what they're really doing;
when one needs you it raises its hand and talks to you in a JRPG dialogue box. Each repo
grows its own party, levels and quest log.

Runs on Linux, macOS and Windows. Everything stays local: no network, no telemetry.

## Install

Needs a Rust toolchain (`cargo`), because herdr builds the plugin on install.

```sh
herdr plugin install KantaKan/herdr-fantasy
```

The stage opens by itself at the bottom of every workspace (`auto_open` in the config
changes that). Try it without herdr: `herdr-fantasy stage --demo`.

## Talk to the agent that needs you

Plugins can't bind keys, so add this to your herdr `config.toml`, then run
`herdr server reload-config`:

```toml
[[keys.command]]
key = "prefix+a"
type = "plugin_action"
command = "kantakan.herdr-fantasy.open-ask"

# optional: show/hide the party
[[keys.command]]
key = "prefix+shift+p"
type = "plugin_action"
command = "kantakan.herdr-fantasy.toggle-stage"
```

When a hero shows ❗, press `prefix+a`: `▶ Go to this agent` jumps to its pane,
`Next agent` cycles if several are waiting, `Esc` closes. The popup never types into
your agents.

## Config

`$HERDR_PLUGIN_CONFIG_DIR/config.toml` (print it with `herdr plugin config-dir
kantakan.herdr-fantasy`). Every key is optional; see `SPEC.md` §9 for the full list.

```toml
auto_open = "every_workspace"   # every_workspace | first_workspace | off
notify_on_blocked = true
notify_sound = "request"        # none | done | request
memes = true
```

`herdr-fantasy doctor` prints what it detected (socket, config, renderer).
