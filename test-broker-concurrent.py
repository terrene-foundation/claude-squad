#!/usr/bin/env python3
"""Concurrent broker test — proves the per-account refresh lock actually
serializes refreshes across OS processes.

Spawns N subprocesses that all fire broker_check() at the same moment.
Each subprocess has its own CLAUDE_CONFIG_DIR pointing at a different
config-X/ directory, but all markers point to the same account. The
canonical credential has expiresAt near-now so every subprocess decides
to refresh.

Expected:
  - exactly ONE subprocess wins the refresh lock and calls refresh_token()
  - all other subprocesses see the lock held and skip
  - after all subprocesses exit, every config-X/.credentials.json has
    the refreshed tokens (via _fan_out_credentials)
  - the canonical credentials/N.json has the refreshed tokens

This exercises the real OS-level flock on POSIX (named mutex on Windows),
not an in-process mock. If the lock leaks or the fanout misses a dir,
the test fails loudly.
"""
import json
import multiprocessing
import os
import shutil
import sys
import tempfile
import time
from pathlib import Path


def child_process(
    config_dir_str,
    counter_file_str,
    refresh_delay_ms,
    accounts_dir_str,
    creds_dir_str,
    engine_path_str,
):
    """Simulates one terminal's statusline render firing broker_check().

    Runs in a subprocess (fork or spawn). Loads the engine fresh, patches
    refresh_token with a counter-incrementing mock, then calls broker_check.
    """
    import fcntl
    import importlib.util
    import json as _json
    import os as _os
    import time as _time
    from pathlib import Path as _Path

    _os.environ["CLAUDE_CONFIG_DIR"] = config_dir_str

    # Load rotation-engine.py fresh in this subprocess
    spec = importlib.util.spec_from_file_location("engine", engine_path_str)
    engine = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(engine)

    # Point the engine at our temp directories
    engine.ACCOUNTS_DIR = _Path(accounts_dir_str)
    engine.CREDS_DIR = _Path(creds_dir_str)
    engine.PROFILES_FILE = _Path(accounts_dir_str) / "profiles.json"
    engine.QUOTA_FILE = _Path(accounts_dir_str) / "quota.json"

    counter_path = _Path(counter_file_str)
    counter_lock = _Path(counter_file_str + ".lock")

    def mock_refresh(account_num, quiet=False):
        # Lock-protected counter increment so we count actual refresh attempts
        with open(counter_lock, "w") as lockf:
            fcntl.flock(lockf, fcntl.LOCK_EX)
            current = int(counter_path.read_text())
            counter_path.write_text(str(current + 1))
            fcntl.flock(lockf, fcntl.LOCK_UN)

        # Simulate Anthropic latency so other subprocesses have a window
        # to try to acquire the broker's refresh-lock and prove it blocks
        _time.sleep(refresh_delay_ms / 1000.0)

        # Write the refreshed credentials to canonical (what real
        # refresh_token does)
        cred_file = engine.CREDS_DIR / f"{account_num}.json"
        new_creds = {
            "claudeAiOauth": {
                "refreshToken": "sk-ant-ort01-REFRESHED",
                "accessToken": "sk-ant-oat01-REFRESHED",
                "expiresAt": int(_time.time() * 1000) + 60 * 60 * 1000,
                "scopes": [],
                "subscriptionType": None,
                "rateLimitTier": None,
            }
        }
        tmp = cred_file.with_suffix(".tmp")
        tmp.write_text(_json.dumps(new_creds, indent=2))
        _os.chmod(tmp, 0o600)
        _os.replace(tmp, cred_file)
        return new_creds

    engine.refresh_token = mock_refresh

    # Fire the broker — this is what the statusline hook does
    engine.broker_check()


def run_test(num_terminals, refresh_delay_ms, engine_path):
    """Spawn num_terminals concurrent broker_check subprocesses."""
    tmpdir = Path(tempfile.mkdtemp(prefix="csq-broker-concurrent-"))
    accounts_dir = tmpdir / "accounts"
    creds_dir = accounts_dir / "credentials"
    creds_dir.mkdir(parents=True)

    # profiles.json so configured_accounts() returns what we expect
    (accounts_dir / "profiles.json").write_text(
        json.dumps({"accounts": {str(i): {"email": f"a{i}@x"} for i in range(1, 8)}})
    )

    # Create N config dirs, all with marker=1
    config_dirs = []
    for i in range(1, num_terminals + 1):
        cd = accounts_dir / f"config-{i}"
        cd.mkdir(parents=True)
        (cd / ".csq-account").write_text("1")
        config_dirs.append(cd)

    # Canonical credential: 5 minutes from expiry (< 10 min threshold)
    now_ms = int(time.time() * 1000)
    initial_creds = {
        "claudeAiOauth": {
            "refreshToken": "sk-ant-ort01-INITIAL",
            "accessToken": "sk-ant-oat01-INITIAL",
            "expiresAt": now_ms + 5 * 60 * 1000,
            "scopes": [],
            "subscriptionType": None,
            "rateLimitTier": None,
        }
    }
    (creds_dir / "1.json").write_text(json.dumps(initial_creds, indent=2))

    # Each terminal has its own live file (same as canonical to start)
    for cd in config_dirs:
        (cd / ".credentials.json").write_text(json.dumps(initial_creds, indent=2))

    # Counter file for tracking refresh_token() calls across processes
    counter_file = tmpdir / "refresh_count"
    counter_file.write_text("0")

    # Spawn all subprocesses as close together as possible
    ctx = multiprocessing.get_context("fork")  # fork lets us share path state
    processes = []
    for cd in config_dirs:
        p = ctx.Process(
            target=child_process,
            args=(
                str(cd),
                str(counter_file),
                refresh_delay_ms,
                str(accounts_dir),
                str(creds_dir),
                engine_path,
            ),
        )
        processes.append(p)

    start = time.time()
    for p in processes:
        p.start()
    for p in processes:
        p.join(timeout=30)
        if p.is_alive():
            p.terminate()
            print(f"  ✗ subprocess {p.pid} hung")
            return False
    duration = time.time() - start

    # ─── Assertions ──────────────────────────────────────
    refresh_count = int(counter_file.read_text())
    canon = json.loads((creds_dir / "1.json").read_text())
    canon_at = canon["claudeAiOauth"]["accessToken"]
    canon_rt = canon["claudeAiOauth"]["refreshToken"]

    print(f"  duration: {duration:.2f}s")
    print(f"  refresh_token() calls: {refresh_count} (expected: 1)")
    print(f"  canonical accessToken: {canon_at}")

    results = []
    results.append(
        (
            "exactly 1 refresh (lock serialized concurrent callers)",
            refresh_count == 1,
        )
    )
    results.append(
        (
            "canonical holds refreshed tokens",
            canon_at == "sk-ant-oat01-REFRESHED"
            and canon_rt == "sk-ant-ort01-REFRESHED",
        )
    )

    # Every config dir should have received the fanout
    for cd in config_dirs:
        live = json.loads((cd / ".credentials.json").read_text())
        live_at = live["claudeAiOauth"]["accessToken"]
        results.append(
            (
                f"{cd.name} received fanout (live == canonical)",
                live_at == "sk-ant-oat01-REFRESHED",
            )
        )

    passed = sum(1 for _, ok in results if ok)
    failed = len(results) - passed
    for name, ok in results:
        icon = "✓" if ok else "✗"
        print(f"  {icon} {name}")

    shutil.rmtree(tmpdir)
    return failed == 0


def run_pullsync_test(engine_path):
    """Terminal A refreshes via broker, terminal B was offline, then B's
    next render runs pullsync and pulls the fresh tokens from canonical.

    This tests the "B is idle when A refreshes" path: B's live file is
    NOT touched by A's fanout because B wasn't running when fanout
    happened. B catches up later via pullsync on its own sync cycle.
    """
    tmpdir = Path(tempfile.mkdtemp(prefix="csq-pullsync-concurrent-"))
    accounts_dir = tmpdir / "accounts"
    creds_dir = accounts_dir / "credentials"
    creds_dir.mkdir(parents=True)

    (accounts_dir / "profiles.json").write_text(
        json.dumps({"accounts": {str(i): {"email": f"a{i}@x"} for i in range(1, 8)}})
    )

    config_a = accounts_dir / "config-1"
    config_b = accounts_dir / "config-2"
    for cd in [config_a, config_b]:
        cd.mkdir(parents=True)
        (cd / ".csq-account").write_text("1")

    now_ms = int(time.time() * 1000)
    # Terminal A's live credentials — stale (what it had before refresh)
    stale_creds = {
        "claudeAiOauth": {
            "refreshToken": "sk-ant-ort01-STALE",
            "accessToken": "sk-ant-oat01-STALE",
            "expiresAt": now_ms + 3 * 60 * 1000,  # 3 min from now
        }
    }
    # Terminal B's live credentials — same stale tokens (was cloned from A)
    (config_b / ".credentials.json").write_text(json.dumps(stale_creds, indent=2))
    # Canonical has FRESH tokens (as if A just refreshed and updated canon)
    fresh_creds = {
        "claudeAiOauth": {
            "refreshToken": "sk-ant-ort01-FRESH",
            "accessToken": "sk-ant-oat01-FRESH",
            "expiresAt": now_ms + 60 * 60 * 1000,  # 1 hour
        }
    }
    (creds_dir / "1.json").write_text(json.dumps(fresh_creds, indent=2))
    # A's live matches canonical (it just refreshed)
    (config_a / ".credentials.json").write_text(json.dumps(fresh_creds, indent=2))

    # Now fire pullsync from terminal B's config dir in a subprocess.
    # B should detect canonical is newer and copy it into B's live.
    def b_pullsync(config_dir_str, accounts_dir_str, creds_dir_str, engine_path_str):
        import importlib.util
        import os as _os
        from pathlib import Path as _Path

        _os.environ["CLAUDE_CONFIG_DIR"] = config_dir_str
        spec = importlib.util.spec_from_file_location("engine", engine_path_str)
        engine = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(engine)
        engine.ACCOUNTS_DIR = _Path(accounts_dir_str)
        engine.CREDS_DIR = _Path(creds_dir_str)
        engine.PROFILES_FILE = _Path(accounts_dir_str) / "profiles.json"
        engine.QUOTA_FILE = _Path(accounts_dir_str) / "quota.json"
        engine.pullsync()

    ctx = multiprocessing.get_context("fork")
    p = ctx.Process(
        target=b_pullsync,
        args=(str(config_b), str(accounts_dir), str(creds_dir), engine_path),
    )
    p.start()
    p.join(timeout=10)
    if p.is_alive():
        p.terminate()
        shutil.rmtree(tmpdir)
        return False

    b_live = json.loads((config_b / ".credentials.json").read_text())
    b_at = b_live["claudeAiOauth"]["accessToken"]
    b_rt = b_live["claudeAiOauth"]["refreshToken"]

    ok = b_at == "sk-ant-oat01-FRESH" and b_rt == "sk-ant-ort01-FRESH"
    icon = "✓" if ok else "✗"
    print(f"  {icon} terminal B pulled fresh tokens from canonical")
    print(f"    B.accessToken: {b_at}")
    print(f"    B.refreshToken: {b_rt}")

    shutil.rmtree(tmpdir)
    return ok


def main():
    engine_path = str((Path(__file__).parent / "rotation-engine.py").resolve())
    if not os.path.exists(engine_path):
        print(f"ERROR: rotation-engine.py not found at {engine_path}")
        sys.exit(1)

    all_passed = True

    for num in [2, 5, 10]:
        print(f"\n=== broker: {num} concurrent subprocesses (100ms delay) ===")
        if not run_test(num, refresh_delay_ms=100, engine_path=engine_path):
            all_passed = False

    print("\n=== pullsync: idle terminal catches up via canonical ===")
    if not run_pullsync_test(engine_path):
        all_passed = False

    print()
    if all_passed:
        print("ALL TESTS PASSED ✓")
        sys.exit(0)
    else:
        print("SOME TESTS FAILED ✗")
        sys.exit(1)


if __name__ == "__main__":
    main()
