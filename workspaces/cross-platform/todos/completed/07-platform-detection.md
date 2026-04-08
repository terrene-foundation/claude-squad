# TODO: Add platform detection constants

**Milestone**: 1 — Engine Cross-Platform
**File**: `rotation-engine.py` (top of file, after imports)
**Blocks**: Todos 08-15
**Blocked by**: None

## What

Add platform detection constants used by all subsequent cross-platform code.

```python
import sys
IS_WINDOWS = sys.platform == "win32"
IS_MACOS = sys.platform == "darwin"
IS_LINUX = sys.platform.startswith("linux")
```

## Acceptance

- Constants available at module level
- `python3 -m py_compile rotation-engine.py` passes
