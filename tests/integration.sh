#!/usr/bin/env bash
# Starts a throwaway herdr server, links the plugin into it, and drives an
# agent's state with `herdr pane report-agent` to check how the mark follows.
#
# Usage: tests/integration.sh PLUGIN_ROOT
# Needs herdr and jq on PATH. Touches no herdr server but its own.
set -euo pipefail

plugin_root=$(realpath "$1")
plugin_id=rheotommy.needs-action

# A short path: the server's socket lives under it, and Unix socket paths are
# limited to about 100 bytes.
work=$(mktemp -d -t hna.XXXXXX)
for name in $(env | grep -o '^HERDR[A-Z_]*' || true); do
  unset "$name"
done
export HOME=$work XDG_CONFIG_HOME=$work/.config XDG_STATE_HOME=$work/.local/state \
  XDG_DATA_HOME=$work/.local/share XDG_RUNTIME_DIR=$work/run
mkdir -p "$XDG_RUNTIME_DIR"

stop_server() {
  herdr server stop >/dev/null 2>&1 || true
  for _ in $(seq 100); do
    herdr status server --json | jq -e '.running | not' >/dev/null && return
    sleep 0.1
  done
}
trap 'stop_server; rm -rf "$work"' EXIT

start_server() {
  herdr server >>"$work/server.log" 2>&1 </dev/null &
  for _ in $(seq 100); do
    herdr status server --json | jq -e .running >/dev/null && return
    sleep 0.1
  done
  echo "herdr server did not start:" >&2
  cat "$work/server.log" >&2
  exit 1
}

fail() {
  echo "FAIL: $*" >&2
  herdr plugin log list --plugin "$plugin_id" --limit 5 | jq . >&2 || true
  exit 1
}

# The number of plugin commands started so far. Every agent status change
# starts the hook once, and every action invocation starts one command.
started=0

# Waits until every started plugin command has finished, so that a check
# after it sees what they wrote.
settle() {
  local logs
  for _ in $(seq 100); do
    logs=$(herdr plugin log list --plugin "$plugin_id" --limit 1000)
    if jq -e '[.result.logs[] | select(.status == "failed")] | length > 0' <<<"$logs" >/dev/null; then
      fail "a plugin command failed"
    fi
    if jq -e --argjson n "$started" '
      (.result.logs | length >= $n) and all(.result.logs[]; .status != "running")
    ' <<<"$logs" >/dev/null; then
      return
    fi
    sleep 0.1
  done
  fail "plugin commands did not finish"
}

report() {
  herdr pane report-agent "$pane" --source test:fake --agent claude --state "$1" >/dev/null
  started=$((started + 1))
  settle
}

invoke() {
  herdr plugin action invoke "$plugin_id.$1" >/dev/null
  started=$((started + 1))
  settle
}

mark() {
  herdr pane get "$pane" | jq -r '.result.pane.tokens.needs_action // ""'
}

expect_marked() {
  [[ -n $(mark) ]] || fail "$1: expected the pane to be marked"
}

expect_unmarked() {
  [[ -z $(mark) ]] || fail "$1: expected no mark, found '$(mark)'"
}

herdr plugin link "$plugin_root" >/dev/null
config_dir=$(herdr plugin config-dir "$plugin_id")
start_server
pane=$(herdr workspace create --cwd "$work" --label test | jq -r .result.root_pane.pane_id)

report idle
expect_unmarked "an agent first seen idle"

report working
report idle
expect_marked "an agent that finished a turn"

other=$(herdr pane split "$pane" --direction right | jq -r .result.pane.pane_id)
herdr pane focus --direction left >/dev/null
[[ $(herdr pane get "$pane" | jq -r .result.pane.focused) == true ]] || fail "could not focus the pane back"
expect_marked "a pane that was looked at again"

report working
expect_unmarked "an agent that started a turn"

report blocked
report idle
expect_unmarked "an agent whose question was turned down"

report working
report blocked
report working
report unknown
report idle
expect_marked "an agent that finished after a question and a detection gap"

invoke toggle
expect_unmarked "a toggled marked pane"
invoke toggle
expect_marked "a toggled unmarked pane"
invoke clear
expect_unmarked "a cleared pane"
invoke mark
expect_marked "a manually marked pane"
[[ $(mark) == "⚑" ]] || fail "expected the default label, found '$(mark)'"

printf 'label = "needs action"\n' >"$config_dir/config.toml"
invoke clear
report working
report idle
[[ $(mark) == "needs action" ]] || fail "expected the configured label, found '$(mark)'"

herdr pane close "$other" >/dev/null
stop_server
start_server
pane=$(herdr pane list | jq -r '.result.panes[0].pane_id')
expect_unmarked "a pane restored after a server restart"

echo "integration: all checks passed"
