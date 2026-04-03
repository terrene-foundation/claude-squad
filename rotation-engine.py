#!/usr/bin/env python3
"""
Claude Squad — Rotation Engine

Tracks quota across accounts and rotates when one is exhausted.
Claude Code handles credential refresh natively on 401 — we just
write .credentials.json and let CC pick it up.

State files (all in ~/.claude/accounts/):
  quota.json           Per-account quota percentages + reset times
  credentials/N.json   Stored OAuth credentials per account (1-7)
  config-N/            Per-account config dir (symlinked settings + own creds)
  profiles.json        Email→account mapping

Commands:
  update              Update quota from statusline JSON (stdin)
  status              Show all accounts and quota
  statusline          Compact string for statusline display
  swap <N>            Write account N's creds to this terminal's config dir
  auto-rotate         Check quota + swap if needed (called from hook)
  auto-rotate --force Mark current exhausted, then rotate
"""

import json
import os
import sys
import time
from pathlib import Path

ACCOUNTS_DIR = Path.home() / ".claude" / "accounts"
CREDS_DIR = ACCOUNTS_DIR / "credentials"
QUOTA_FILE = ACCOUNTS_DIR / "quota.json"
PROFILES_FILE = ACCOUNTS_DIR / "profiles.json"
MAX_ACCOUNTS = 7
GLOBAL_CLAUDE_DIR = Path.home() / ".claude"


# ─── State ───────────────────────────────────────────────


def _load(path, default):
    try:
        return json.loads(path.read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return default


def _save(path, data):
    path.write_text(json.dumps(data, indent=2))


def load_state():
    state = _load(QUOTA_FILE, {"accounts": {}})
    # Reset expired quotas
    now = time.time()
    for acct_data in state.get("accounts", {}).values():
        for window in ("five_hour", "seven_day"):
            w = acct_data.get(window, {})
            resets_at = w.get("resets_at", 0)
            if resets_at and resets_at < now and w.get("used_percentage", 0) > 0:
                w["used_percentage"] = 0
    return state


def get_email(n):
    return _load(PROFILES_FILE, {}).get("accounts", {}).get(str(n), {}).get("email", "")


def configured_accounts():
    """List account numbers that have credentials."""
    return [
        str(n) for n in range(1, MAX_ACCOUNTS + 1) if (CREDS_DIR / f"{n}.json").exists()
    ]


# ─── This Terminal ───────────────────────────────────────


def this_account():
    """Which account is this terminal on? Derived from CLAUDE_CONFIG_DIR name."""
    config_dir = os.environ.get("CLAUDE_CONFIG_DIR", "")
    if not config_dir:
        return None
    name = Path(config_dir).name
    if name.startswith("config-") and name[7:].isdigit():
        return name[7:]
    return None


# ─── Pick Best ───────────────────────────────────────────


def pick_best(state, exclude=None):
    """Pick the account with the most available quota.
    Prefers accounts with lower 5h usage. When all exhausted,
    picks the one whose reset is soonest."""
    now = time.time()
    available = []
    exhausted = []

    for n in configured_accounts():
        if n == str(exclude):
            continue
        acct = state.get("accounts", {}).get(n, {})
        five = acct.get("five_hour", {})
        seven = acct.get("seven_day", {})
        five_pct = five.get("used_percentage", 0)
        seven_pct = seven.get("used_percentage", 0)
        five_reset = five.get("resets_at", 0)

        if five_pct >= 100 or seven_pct >= 100:
            exhausted.append((n, five_reset))
            continue
        available.append((n, five_pct))

    if available:
        available.sort(key=lambda x: x[1])  # lowest usage first
        return available[0][0]

    if exhausted:
        future = [(n, r) for n, r in exhausted if r > now]
        if future:
            future.sort(key=lambda x: x[1])  # soonest reset first
            return future[0][0]

    return None


# ─── Swap ────────────────────────────────────────────────


def swap_to(target_account):
    """Write target account's credentials to this terminal's config dir.
    Claude Code picks up new creds on next 401."""
    target_account = str(target_account)
    source_cred = CREDS_DIR / f"{target_account}.json"
    if not source_cred.exists():
        print(f"error: no credentials for account {target_account}", file=sys.stderr)
        return False

    config_dir = os.environ.get("CLAUDE_CONFIG_DIR", "")
    if not config_dir:
        print("error: CLAUDE_CONFIG_DIR not set — launch via 'cc <N>'", file=sys.stderr)
        return False

    target_path = Path(config_dir) / ".credentials.json"
    target_path.write_text(source_cred.read_text())
    target_path.chmod(0o600)

    email = get_email(target_account)
    print(f"Swapped to account {target_account} ({email})")
    return True


# ─── Quota Update ────────────────────────────────────────


def update_quota(json_str):
    """Called from statusline. Updates quota for this terminal's account.
    Auto-rotates at 100%."""
    try:
        data = json.loads(json_str)
    except json.JSONDecodeError:
        return

    rate_limits = data.get("rate_limits")
    if not rate_limits:
        return

    current = this_account()
    if not current:
        return

    state = load_state()
    state.setdefault("accounts", {})[current] = {
        "five_hour": rate_limits.get("five_hour", {}),
        "seven_day": rate_limits.get("seven_day", {}),
        "updated_at": time.time(),
    }
    _save(QUOTA_FILE, state)

    # Auto-rotate at 100%
    five_pct = rate_limits.get("five_hour", {}).get("used_percentage", 0)
    if five_pct >= 100:
        target = pick_best(state, exclude=current)
        if target and swap_to(target):
            print(
                f"[auto-rotate] → account {target} ({get_email(target)})",
                file=sys.stderr,
            )


# ─── Auto-Rotate (hook) ─────────────────────────────────


def auto_rotate(force=False):
    """Called from UserPromptSubmit hook as backup.
    Primary rotation happens in update_quota (statusline)."""
    current = this_account()
    if not current:
        return

    state = load_state()

    if force:
        state.setdefault("accounts", {}).setdefault(current, {})["five_hour"] = {
            "used_percentage": 100,
            "resets_at": time.time() + 18000,
        }
        _save(QUOTA_FILE, state)

    acct = state.get("accounts", {}).get(current, {})
    five_pct = acct.get("five_hour", {}).get("used_percentage", 0)

    if five_pct >= 100 or force:
        target = pick_best(state, exclude=current)
        if target:
            if swap_to(target):
                print(
                    f"[auto-rotate] → account {target} ({get_email(target)})",
                    file=sys.stderr,
                )
        elif force:
            show_status()
            print("\nAll accounts exhausted.")


# ─── Status ──────────────────────────────────────────────


def fmt_time(epoch):
    diff = epoch - time.time()
    if diff <= 0:
        return "now"
    h, m = int(diff // 3600), int((diff % 3600) // 60)
    if h >= 24:
        return f"{h // 24}d{h % 24}h"
    return f"{h}h{m}m" if h > 0 else f"{m}m"


def show_status():
    state = load_state()
    current = this_account()

    print(
        f"Claude Squad — this terminal: account {current} ({get_email(current)})"
        if current
        else "Claude Squad — not launched via cc"
    )
    print("=" * 50)

    for n in configured_accounts():
        acct = state.get("accounts", {}).get(n, {})
        email = get_email(n)
        marker = "→" if n == current else " "
        five = acct.get("five_hour", {})
        seven = acct.get("seven_day", {})
        five_pct = five.get("used_percentage", 0)
        seven_pct = seven.get("used_percentage", 0)
        five_reset = five.get("resets_at", 0)
        seven_reset = seven.get("resets_at", 0)

        icon = "●" if five_pct < 80 else ("◐" if five_pct < 100 else "○")
        print(f" {marker} {n}  {icon} {email}")
        if acct:
            r5 = fmt_time(five_reset) if five_reset else "?"
            r7 = fmt_time(seven_reset) if seven_reset else "?"
            print(f"       5h:{five_pct:.0f}% ↻{r5}  7d:{seven_pct:.0f}% ↻{r7}")
    print()


def statusline_str():
    current = this_account()
    if not current:
        return ""
    state = load_state()
    acct = state.get("accounts", {}).get(current, {})
    email = get_email(current)
    user = email.split("@")[0][:10] if email else ""
    five_pct = acct.get("five_hour", {}).get("used_percentage", 0)
    seven_pct = acct.get("seven_day", {}).get("used_percentage", 0)
    parts = [f"#{current}:{user}"]
    if five_pct > 0 or seven_pct > 0:
        parts.append(f"5h:{five_pct:.0f}%")
        parts.append(f"7d:{seven_pct:.0f}%")
    return " ".join(parts)


# ─── Main ────────────────────────────────────────────────


def main():
    cmd = sys.argv[1] if len(sys.argv) > 1 else "status"

    if cmd == "status":
        show_status()
    elif cmd == "update":
        update_quota(sys.stdin.read())
    elif cmd == "swap":
        if len(sys.argv) < 3:
            print("usage: rotation-engine.py swap <N>", file=sys.stderr)
            sys.exit(1)
        if not swap_to(sys.argv[2]):
            sys.exit(1)
    elif cmd == "auto-rotate":
        auto_rotate(force="--force" in sys.argv)
    elif cmd == "statusline":
        print(statusline_str())
    elif cmd == "check":
        current = this_account()
        state = load_state()
        if not current:
            print(json.dumps({"should_rotate": False}))
            sys.exit(0)
        acct = state.get("accounts", {}).get(current, {})
        five_pct = acct.get("five_hour", {}).get("used_percentage", 0)
        should = five_pct >= 100
        target = pick_best(state, exclude=current) if should else None
        print(
            json.dumps(
                {"should_rotate": should and target is not None, "target": target}
            )
        )
    elif cmd == "which":
        current = this_account()
        if current:
            print(f"account {current} ({get_email(current)})")
        else:
            print("not launched via cc (no CLAUDE_CONFIG_DIR)")
    else:
        print(f"Unknown command: {cmd}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
