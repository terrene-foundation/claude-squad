#!/usr/bin/env bash
# cc — Claude Code multi-account launcher
# Usage:
#   cc                   Launch with default account
#   cc 1-7               Launch with specific account
#   cc login <N>         Login to account N (browser)
#   cc status            Show all accounts + quota
#   cc swap <N>          Swap to account N mid-session
#   cc <any args>        Pass through to claude

set -euo pipefail

ACCOUNTS_DIR="$HOME/.claude/accounts"
ENGINE="$ACCOUNTS_DIR/rotation-engine.py"

die() { echo "error: $*" >&2; exit 1; }

# ─── Launch ───────────────────────────────────────────────

launch_account() {
  local n="$1"
  shift
  local config_dir="$ACCOUNTS_DIR/config-${n}"
  [[ -d "$config_dir" ]] || die "account $n not set up. Run: cc login $n"

  export CLAUDE_CONFIG_DIR="$config_dir"

  # Copy fresh credentials into config dir
  local cred_file="$ACCOUNTS_DIR/credentials/${n}.json"
  if [[ -f "$cred_file" ]]; then
    cp "$cred_file" "$config_dir/.credentials.json"
    chmod 600 "$config_dir/.credentials.json"
  fi

  exec claude "$@"
}

# ─── Login ────────────────────────────────────────────────

cmd_login() {
  local n="$1"
  [[ "$n" =~ ^[1-7]$ ]] || die "account must be 1-7, got: $n"

  local config_dir="$ACCOUNTS_DIR/config-${n}"
  mkdir -p "$config_dir"

  # Symlink shared state to global ~/.claude/
  for item in settings.json projects plugins sessions; do
    local target="$HOME/.claude/$item"
    local link="$config_dir/$item"
    if [[ -e "$target" ]] && [[ ! -L "$link" ]]; then
      [[ -d "$link" ]] && rm -rf "$link"
      [[ -e "$link" ]] && rm -f "$link"
      ln -sf "$target" "$link"
    fi
  done

  echo "Logging in to account $n..."
  echo "A browser will open. Sign in with the account you want for slot $n."
  echo ""

  # Clean slate for fresh login
  rm -f "$config_dir/.credentials.json"
  CLAUDE_CONFIG_DIR="$config_dir" claude auth logout 2>/dev/null || true
  CLAUDE_CONFIG_DIR="$config_dir" claude auth login || die "login failed"

  # Get the email
  local email
  email=$(CLAUDE_CONFIG_DIR="$config_dir" claude auth status --json 2>/dev/null \
    | python3 -c "import json,sys; print(json.load(sys.stdin).get('email','unknown'))" 2>/dev/null \
    || echo "unknown")

  # Save profile
  python3 -c "
import json
f = '$ACCOUNTS_DIR/profiles.json'
try:
    d = json.load(open(f))
except:
    d = {'accounts': {}}
d.setdefault('accounts', {})['$n'] = {'email': '$email', 'method': 'oauth'}
with open(f, 'w') as fh:
    json.dump(d, fh, indent=2)
" 2>/dev/null

  # Copy credentials to pool
  mkdir -p "$ACCOUNTS_DIR/credentials"
  if [[ -f "$config_dir/.credentials.json" ]]; then
    cp "$config_dir/.credentials.json" "$ACCOUNTS_DIR/credentials/${n}.json"
    chmod 600 "$ACCOUNTS_DIR/credentials/${n}.json"
  else
    echo "warning: no .credentials.json after login" >&2
  fi

  # Copy onboarding state so CC doesn't show first-run flow
  local home_json="$HOME/.claude.json"
  local conf_json="$config_dir/.claude.json"
  if [[ -f "$home_json" ]] && [[ -f "$conf_json" ]]; then
    python3 -c "
import json
home = json.load(open('$home_json'))
conf = json.load(open('$conf_json'))
for k in ['hasCompletedOnboarding', 'lastOnboardingVersion', 'numStartups',
           'migrationVersion', 'hasAvailableSubscription', 'hasAvailableMaxSubscription']:
    if k in home and k not in conf:
        conf[k] = home[k]
with open('$conf_json', 'w') as f:
    json.dump(conf, f)
" 2>/dev/null
  fi

  echo ""
  echo "Account $n ($email) ready. Launch with: cc $n"
}

# ─── Status ───────────────────────────────────────────────

cmd_status() {
  python3 "$ENGINE" status
}

# ─── Main ─────────────────────────────────────────────────

main() {
  local cmd="${1:-}"

  case "$cmd" in
    "")
      exec claude
      ;;
    [1-7])
      shift
      launch_account "$cmd" "$@"
      ;;
    login)
      shift
      [[ $# -ge 1 ]] || die "usage: cc login <1-7>"
      cmd_login "$1"
      ;;
    swap)
      shift
      [[ $# -ge 1 ]] || die "usage: cc swap <1-7>"
      [[ "$1" =~ ^[1-7]$ ]] || die "account must be 1-7, got: $1"
      python3 "$ENGINE" swap "$1"
      ;;
    status|ls)
      cmd_status
      ;;
    quota)
      python3 "$ENGINE" status
      ;;
    help|-h|--help)
      cat <<'HELP'
cc — Claude Code multi-account launcher

LAUNCH:
  cc                   Launch with default account
  cc 1-7               Launch with specific account

MANAGE:
  cc login <N>         Login to account N (opens browser)
  cc status            Show all accounts + quota
  cc swap <N>          Swap to account N mid-session
  cc help              This message

SETUP:
  1. cc login 1        Login to first account
  2. cc login 2        Login to second account
  3. ...               Add up to 7 accounts
  4. cc 1              Launch with account 1

Auto-rotation happens automatically when a rate limit hits.
Use /rotate inside Claude Code to force-rotate.
HELP
      ;;
    *)
      # Pass through to claude
      exec claude "$@"
      ;;
  esac
}

main "$@"
