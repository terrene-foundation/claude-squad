#!/usr/bin/env bash
# Cross-platform smoke test for claude-squad.
# Runs in CI on macOS, Linux (Ubuntu), and Windows (Git Bash).
# Each test case prints PASS/FAIL with diagnostic info on failure.

set -uo pipefail

PASS=0
FAIL=0

pass() { echo "  PASS: $1"; PASS=$((PASS+1)); }
fail() { echo "  FAIL: $1 -- $2"; FAIL=$((FAIL+1)); }

# Resolve repo root (this script lives at the repo root)
REPO_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO_DIR"

# Resolve Python 3
find_python() {
    for cmd in python3 python py; do
        if command -v "$cmd" >/dev/null 2>&1 && "$cmd" --version 2>&1 | grep -q "Python 3"; then
            echo "$cmd"
            return
        fi
    done
    return 1
}

detect_platform() {
    case "$(uname -s)" in
        Darwin) echo "macos" ;;
        Linux) grep -qi microsoft /proc/version 2>/dev/null && echo "wsl" || echo "linux" ;;
        MINGW*|MSYS*|CYGWIN*) echo "git-bash" ;;
        *) echo "unknown" ;;
    esac
}

PLATFORM=$(detect_platform)
echo "===================================================="
echo "claude-squad smoke test"
echo "  platform: $PLATFORM"
echo "  repo:     $REPO_DIR"
echo "===================================================="
echo

# ─── Test 1: Python detection (bash) ──────────────────
echo "[1] Python detection (bash)"
PY=$(find_python) && pass "found $PY" || fail "no Python 3" "tried python3, python, py"

# ─── Test 2: Engine python-cmd ─────────────────────────
echo "[2] Engine reports its Python command"
if [ -n "${PY:-}" ]; then
    engine_py=$("$PY" rotation-engine.py python-cmd 2>&1)
    [ -n "$engine_py" ] && pass "engine reports: $engine_py" || fail "engine returned empty" "$engine_py"
else
    fail "skipped" "no Python"
fi

# ─── Test 3: Engine compiles ──────────────────────────
echo "[3] Engine compiles"
if [ -n "${PY:-}" ]; then
    if "$PY" -m py_compile rotation-engine.py 2>&1; then
        pass "py_compile clean"
    else
        fail "compile error"
    fi
fi

# ─── Test 4: Bash scripts have valid syntax ──────────
echo "[4] Bash scripts pass syntax check"
for f in csq install.sh statusline-quota.sh; do
    if bash -n "$f" 2>&1; then
        pass "$f syntax OK"
    else
        fail "$f syntax error"
    fi
done

# ─── Test 5: All shell scripts use #!/usr/bin/env bash ─
echo "[5] All .sh shebangs are #!/usr/bin/env bash"
for f in csq install.sh statusline-quota.sh test-platform.sh; do
    line=$(head -1 "$f")
    if [ "$line" = "#!/usr/bin/env bash" ]; then
        pass "$f"
    else
        fail "$f" "got '$line'"
    fi
done

# ─── Test 6: No bare 'python3' command in scripts ─────
echo "[6] No hardcoded 'python3' command in scripts"
# Allow comments and 'cmd in python3 python py' loops, but not literal command invocation
bad=$(grep -nE '(^|[^"$_a-zA-Z])python3 ' csq statusline-quota.sh 2>/dev/null \
    | grep -v '#' \
    | grep -v 'cmd in python3' \
    || true)
if [ -z "$bad" ]; then
    pass "all scripts use \$PY"
else
    fail "hardcoded python3 found" "$bad"
fi

# ─── Test 7: No 'bc' dependency in statusline ────────
echo "[7] statusline-quota.sh does not depend on bc"
if grep -q 'bc$\| bc ' statusline-quota.sh; then
    fail "bc found" "$(grep -n bc statusline-quota.sh)"
else
    pass "bc not used (awk only)"
fi

# ─── Test 8: Engine status command runs ──────────────
echo "[8] rotation-engine.py status runs without crash"
if [ -n "${PY:-}" ]; then
    out=$("$PY" rotation-engine.py status 2>&1)
    if [ $? -eq 0 ]; then
        pass "status exited 0"
    else
        fail "status crashed" "$out"
    fi
fi

# ─── Test 9: Atomic rename works ─────────────────────
echo "[9] _atomic_replace works in isolation"
if [ -n "${PY:-}" ]; then
    out=$("$PY" -c "
import sys
sys.path.insert(0, '.')
import importlib.util
spec = importlib.util.spec_from_file_location('engine', 'rotation-engine.py')
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
import tempfile, os
with tempfile.TemporaryDirectory() as d:
    target = os.path.join(d, 'target')
    open(target, 'w').write('OLD')
    tmp = os.path.join(d, 'tmp')
    open(tmp, 'w').write('NEW')
    m._atomic_replace(tmp, target)
    assert open(target).read() == 'NEW', 'replace failed'
    print('OK')
" 2>&1)
    if [ "$out" = "OK" ]; then
        pass "_atomic_replace overwrote target"
    else
        fail "_atomic_replace failed" "$out"
    fi
fi

# ─── Test 10: PID detection — current process is alive ─
echo "[10] _is_pid_alive($$) returns True for current PID"
if [ -n "${PY:-}" ]; then
    out=$("$PY" -c "
import sys
sys.path.insert(0, '.')
import importlib.util
spec = importlib.util.spec_from_file_location('engine', 'rotation-engine.py')
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
import os
print('alive' if m._is_pid_alive(os.getppid()) else 'dead')
" 2>&1)
    if [ "$out" = "alive" ]; then
        pass "PID alive detected"
    else
        fail "_is_pid_alive returned $out"
    fi

    # And a definitely-dead PID
    out2=$("$PY" -c "
import sys
sys.path.insert(0, '.')
import importlib.util
spec = importlib.util.spec_from_file_location('engine', 'rotation-engine.py')
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
print('alive' if m._is_pid_alive(99999) else 'dead')
" 2>&1)
    if [ "$out2" = "dead" ]; then
        pass "PID 99999 dead detected"
    else
        fail "PID 99999 reported as $out2"
    fi
fi

# ─── Test 11: Process tree finder doesn't crash ──────
echo "[11] _find_cc_pid() runs without crash (returns None when CC not running)"
if [ -n "${PY:-}" ]; then
    out=$("$PY" -c "
import sys
sys.path.insert(0, '.')
import importlib.util
spec = importlib.util.spec_from_file_location('engine', 'rotation-engine.py')
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
result = m._find_cc_pid()
print('OK' if result is None or isinstance(result, int) else f'BAD: {result}')
" 2>&1)
    if [ "$out" = "OK" ]; then
        pass "_find_cc_pid returned None or int"
    else
        fail "_find_cc_pid bad result" "$out"
    fi
fi

# ─── Test 12: File locking abstraction ───────────────
echo "[12] _lock_file/_unlock_file round-trip"
if [ -n "${PY:-}" ]; then
    out=$("$PY" -c "
import sys
sys.path.insert(0, '.')
import importlib.util
spec = importlib.util.spec_from_file_location('engine', 'rotation-engine.py')
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
import tempfile, os
with tempfile.TemporaryDirectory() as d:
    lock = os.path.join(d, 'test.lock')
    h = m._lock_file(lock)
    if h is None:
        print('FAIL: lock returned None')
    else:
        m._unlock_file(h)
        print('OK')
" 2>&1)
    if [ "$out" = "OK" ]; then
        pass "lock/unlock round-trip"
    else
        fail "lock failed" "$out"
    fi
fi

# ─── Test 13: Concurrent locking serializes (catches C1 ctypes truncation) ─
echo "[13] Two concurrent _lock_file calls serialize correctly"
if [ -n "${PY:-}" ]; then
    out=$("$PY" -c "
import sys, os, tempfile, threading, time
sys.path.insert(0, '.')
import importlib.util
spec = importlib.util.spec_from_file_location('engine', 'rotation-engine.py')
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

# Test: two threads racing on the same lock should serialize.
# If the lock is broken (e.g., truncated handle on Windows),
# both threads will hold the 'lock' simultaneously and the
# critical_section_count will exceed 1.
with tempfile.TemporaryDirectory() as d:
    lock = os.path.join(d, 'test.lock')
    in_critical = [0]
    max_observed = [0]
    errors = []

    def worker():
        try:
            for _ in range(20):
                h = m._lock_file(lock)
                if h is None:
                    errors.append('lock returned None')
                    return
                in_critical[0] += 1
                if in_critical[0] > max_observed[0]:
                    max_observed[0] = in_critical[0]
                time.sleep(0.001)
                in_critical[0] -= 1
                m._unlock_file(h)
        except Exception as e:
            errors.append(str(e))

    threads = [threading.Thread(target=worker) for _ in range(4)]
    for t in threads: t.start()
    for t in threads: t.join()

    if errors:
        print(f'FAIL: {errors[0]}')
    elif max_observed[0] > 1:
        print(f'FAIL: {max_observed[0]} threads in critical section simultaneously')
    else:
        print('OK')
" 2>&1)
    if [ "$out" = "OK" ]; then
        pass "concurrent locking serializes"
    else
        fail "concurrent lock broken" "$out"
    fi
fi

# ─── Test 14: Stuck-swap regression (marker lies, content is truth) ─
# Regression for: after `csq swap N`, CC can hold an old refresh token in
# memory and write it back to .credentials.json. The markers then claim
# account N while CC is actually running the old account. update_quota
# must attribute the live rate_limits to the CONTENT-matched account (not
# the marker-claimed one), and statusline must flag the mismatch.
echo "[14] Stuck-swap regression: content-match overrides stale markers"
if [ -n "${PY:-}" ]; then
    out=$("$PY" -c "
import sys, os, tempfile, json
sys.path.insert(0, '.')
import importlib.util

with tempfile.TemporaryDirectory() as tmp:
    # Build a fake HOME so ACCOUNTS_DIR / CREDS_DIR / QUOTA_FILE land here
    os.environ['HOME'] = tmp
    os.environ['USERPROFILE'] = tmp  # Windows
    accounts = os.path.join(tmp, '.claude', 'accounts')
    creds = os.path.join(accounts, 'credentials')
    config_dir = os.path.join(accounts, 'config-1')
    os.makedirs(creds, exist_ok=True)
    os.makedirs(config_dir, exist_ok=True)

    # Two canonical accounts with distinct refresh tokens
    def creds_blob(refresh, access):
        return {'claudeAiOauth': {'refreshToken': refresh, 'accessToken': access}}
    with open(os.path.join(creds, '1.json'), 'w') as f:
        json.dump(creds_blob('refresh-ONE', 'access-ONE'), f)
    with open(os.path.join(creds, '2.json'), 'w') as f:
        json.dump(creds_blob('refresh-TWO', 'access-TWO'), f)

    # Profiles so configured_accounts() sees 1 and 2
    with open(os.path.join(accounts, 'profiles.json'), 'w') as f:
        json.dump({'accounts': {'1': {'email': 'one@x'}, '2': {'email': 'two@x'}}}, f)

    # STUCK STATE: live .credentials.json holds account 2's refresh token
    # (CC is running account 2), but the markers insist account 1.
    with open(os.path.join(config_dir, '.credentials.json'), 'w') as f:
        json.dump(creds_blob('refresh-TWO', 'access-TWO-refreshed'), f)
    with open(os.path.join(config_dir, '.csq-account'), 'w') as f:
        f.write('1')
    with open(os.path.join(config_dir, '.current-account'), 'w') as f:
        f.write('1')
    os.environ['CLAUDE_CONFIG_DIR'] = config_dir

    # Import the engine AFTER env is set so Path.home() binds to tmp
    spec = importlib.util.spec_from_file_location('engine', 'rotation-engine.py')
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)

    # (a) live_credentials_account returns the CONTENT-matched account
    live = m.live_credentials_account()
    if live != '2':
        print(f'FAIL: live_credentials_account returned {live!r}, expected 2')
        sys.exit(1)

    # (b) which_account returns the stale marker value
    marker = m.which_account()
    if marker != '1':
        print(f'FAIL: which_account returned {marker!r}, expected 1')
        sys.exit(1)

    # (c) update_quota attributes the rate_limits to account 2 (content),
    #     NOT account 1 (marker)
    # load_state() auto-zeros windows whose resets_at is in the past,
    # so use a timestamp well in the future.
    import time as _t
    future = int(_t.time()) + 3600
    payload = json.dumps({'rate_limits': {
        'five_hour':  {'used_percentage': 77, 'resets_at': future},
        'seven_day':  {'used_percentage': 88, 'resets_at': future},
    }})
    m.update_quota(payload)
    quota = json.loads(open(m.QUOTA_FILE).read())
    accts = quota.get('accounts', {})
    if '1' in accts:
        print(f'FAIL: account 1 was contaminated with {accts[\"1\"]}')
        sys.exit(1)
    if accts.get('2', {}).get('five_hour', {}).get('used_percentage') != 77:
        print(f'FAIL: account 2 was not updated correctly, got {accts.get(\"2\")}')
        sys.exit(1)

    # (d) statusline_str shows marker label (user's intent) with a tiny ⚠
    # warning flag that CC is actually running a different account.
    s = m.statusline_str()
    if not s.startswith('#1'):
        print(f'FAIL: statusline label not #1 (marker), got {s!r}')
        sys.exit(1)
    if '⚠' not in s:
        print(f'FAIL: statusline missing warning flag ⚠, got {s!r}')
        sys.exit(1)

    print('OK')
" 2>&1)
    if [ "$out" = "OK" ]; then
        pass "content-match overrides stale markers"
    else
        fail "stuck-swap regression" "$out"
    fi
fi

# ─── Summary ─────────────────────────────────────────
echo
echo "===================================================="
echo "  PASS: $PASS"
echo "  FAIL: $FAIL"
echo "===================================================="

[ $FAIL -eq 0 ]
