# TODO: Add Python command detection helper

**Milestone**: 1 — Engine Cross-Platform
**File**: `rotation-engine.py`
**Blocks**: Todo 18 (install.sh python detection)
**Blocked by**: Todo 07 (platform detection)

## What

Add `_python_cmd()` that returns the correct Python 3 invocation for the current platform.

On Windows, Python is `python` or `py -3`, not `python3`. This function tries candidates and returns the first that works.

```python
def _python_cmd():
    if not IS_WINDOWS:
        return "python3"
    for cmd in ["python3", "python", "py"]:
        try:
            r = subprocess.run([cmd, "--version"], capture_output=True, text=True, timeout=3)
            if r.returncode == 0 and "Python 3" in r.stdout:
                return cmd
        except FileNotFoundError:
            continue
    return "python"
```

Also expose via CLI: `rotation-engine.py python-cmd` prints the resolved command for shell scripts to use.

## Acceptance

- `python3 rotation-engine.py python-cmd` prints `python3` on macOS/Linux
- On Windows: prints `python` or `py` depending on what's installed
