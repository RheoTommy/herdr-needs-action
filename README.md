# herdr-needs-action

A [herdr](https://herdr.dev) plugin that marks an agent pane as needing
action when the agent finishes its work. The mark stays until you start the
agent again or clear the mark yourself.

herdr's built-in `done` state returns to `idle` as soon as you view the
pane. That works well as an unread marker, but it cannot track work that you
have seen and still need to handle. This plugin's mark stays even after you
view the pane.

## How the mark works

| What happens in the pane | Mark |
| :--- | :--- |
| The agent transitions from `working` to `idle` or `done` | Set |
| The agent starts working | Cleared |
| You view the pane | Kept |
| The agent asks a question (`blocked`) | Kept |
| You decline the question, and the agent transitions from `blocked` to `idle` | Not set |
| You run the `toggle`, `mark`, or `clear` action | Toggled, set, or cleared |
| The herdr server restarts | Gone |

The mark is a pane token named `needs_action`. herdr discards a pane's
tokens when the pane closes and does not restore them after a server
restart, so marks never carry over from an earlier server.

## Install

This plugin requires herdr 0.9.5 or later on Linux or macOS.

### With Nix

The flake's default package places the plugin under
`share/herdr-plugins/needs-action`. Build the package and link that
directory:

```sh
herdr plugin link "$(nix build github:RheoTommy/herdr-needs-action --print-out-paths)/share/herdr-plugins/needs-action"
```

herdr records the resolved store path, so relink the new path after each
update. With Home Manager, you can use a `home.activation` script to update
the link on every switch.

### From GitHub

If you have a Rust toolchain on `PATH`, herdr can build and install the
plugin directly:

```sh
herdr plugin install RheoTommy/herdr-needs-action
```

## Show the mark

The plugin only sets the token. To display it, add `$needs_action` to the
Agent sidebar rows in herdr's `config.toml`:

```toml
[ui.sidebar.agents]
rows = [
  ["state_icon", "machine", "workspace", "tab"],
  ["agent", { token = "$needs_action", fg = "#d20f39", bold = true }],
]
```

To set or clear the mark manually, bind a key to the `toggle` action:

```toml
[[keys.command]]
key = "prefix+shift+a"
type = "plugin_action"
command = "rheotommy.needs-action.toggle"
description = "toggle the needs-action mark"
```

The `rheotommy.needs-action.mark` and `rheotommy.needs-action.clear` actions
set and clear the mark without toggling it. Each action applies to the
focused pane.

When you attach with `herdr --remote`, these settings are split between the
two machines. The sidebar rows come from the client's `config.toml`, so
configure them on the client. Because herdr does not send the client's
`[[keys.command]]` bindings to the server, add the keybinding to the
server's `config.toml`, where the plugin runs. The client then uses that
binding only when you attach with `--remote-keybindings server`.

## Configure

By default, the mark displays a flag (`⚑`). To change it, create a
`config.toml` file in the directory printed by
`herdr plugin config-dir rheotommy.needs-action`:

```toml
label = "needs action"
```

A new label applies to marks set after the change.

## Develop

This repository is a Cargo workspace with two crates:

- `crates/needs-action` determines when a pane gets or loses the mark. It
  knows nothing about herdr.
- `crates/herdr-needs-action` is the command that herdr runs. It reads
  events and tokens from herdr, passes them to `needs-action`, and writes
  the result back as tokens.

`nix develop` opens a shell with Cargo and the herdr binary that the tests
use.

- `cargo test` runs the unit tests.
- `nix flake check` also runs `tests/integration.sh`. The script starts a
  separate herdr server in a temporary directory, links the built plugin,
  and drives an agent's state with `herdr pane report-agent`.

To test a change in your own herdr instance, build the plugin and link your
local checkout:

```sh
cargo build --release
herdr plugin link .
```
