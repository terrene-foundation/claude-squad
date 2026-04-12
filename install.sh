#!/usr/bin/env bash
set -euo pipefail

# Code Session Quota (csq) installer — multi-account rotation for Claude Code
# macOS / Linux / WSL / Git Bash on Windows
# Install: curl -sSL https://raw.githubusercontent.com/terrene-foundation/csq/main/install.sh | bash

REPO_URL="https://raw.githubusercontent.com/terrene-foundation/csq/main"
ACCOUNTS_DIR="$HOME/.claude/accounts"
if [[ -d "$HOME/bin" ]] && echo "$PATH" | grep -q "$HOME/bin"; then
    BIN_DIR="$HOME/bin"
else
    BIN_DIR="$HOME/.local/bin"
fi

GREEN='\033[0;32m'; YELLOW='\033[1;33m'; RED='\033[0;31m'; BOLD='\033[1m'; NC='\033[0m'
ok()   { echo -e "${GREEN}✓${NC} $*"; }
warn() { echo -e "${YELLOW}!${NC} $*"; }
err()  { echo -e "${RED}✗${NC} $*" >&2; }

# ─── Platform detection ─────────────────────────────────
detect_platform() {
    case "$(uname -s)" in
        Darwin) echo "macos" ;;
        Linux)
            if grep -qi microsoft /proc/version 2>/dev/null; then
                echo "wsl"
            else
                echo "linux"
            fi ;;
        MINGW*|MSYS*|CYGWIN*) echo "git-bash" ;;
        *) echo "unknown" ;;
    esac
}
PLATFORM=$(detect_platform)

echo -e "\n${BOLD}Claude Squad — Multi-Account Rotation for Claude Code${NC}\n"
echo -e "Platform: ${BOLD}${PLATFORM}${NC}\n"

# ─── Prerequisites ──────────────────────────────────────
command -v claude &>/dev/null || { err "Claude Code not found. Install: https://docs.anthropic.com/en/docs/claude-code"; exit 1; }

if ! command -v jq &>/dev/null; then
    warn "jq not found — statusline will not show quota."
fi

mkdir -p "$ACCOUNTS_DIR/credentials" "$BIN_DIR"

# chmod is a no-op on Windows (NTFS profile dirs are user-private already)
if [[ "$PLATFORM" != "git-bash" ]]; then
    chmod 700 "$ACCOUNTS_DIR" "$ACCOUNTS_DIR/credentials"
fi

# ─── Install files ──────────────────────────────────────
if [[ -f "$(dirname "$0")/statusline-quota.sh" ]]; then
    SRC="$(cd "$(dirname "$0")" && pwd)"
    cp "$SRC/statusline-quota.sh" "$ACCOUNTS_DIR/statusline-quota.sh"
    # Copy the csq binary if built
    if [[ -x "$SRC/target/debug/csq-cli" ]]; then
        cp "$SRC/target/debug/csq-cli" "$BIN_DIR/csq"
    elif [[ -x "$SRC/target/release/csq-cli" ]]; then
        cp "$SRC/target/release/csq-cli" "$BIN_DIR/csq"
    fi
    for doc in 3p-model-primer.md 3p-model-primer-prepend.md; do
        [[ -f "$SRC/$doc" ]] && cp "$SRC/$doc" "$ACCOUNTS_DIR/$doc"
    done
else
    curl -sfL "$REPO_URL/statusline-quota.sh" -o "$ACCOUNTS_DIR/statusline-quota.sh"
    curl -sfL "$REPO_URL/3p-model-primer.md" -o "$ACCOUNTS_DIR/3p-model-primer.md"
    curl -sfL "$REPO_URL/3p-model-primer-prepend.md" -o "$ACCOUNTS_DIR/3p-model-primer-prepend.md"
fi

# auto-rotate-hook.sh: write a no-op so any pre-existing UserPromptSubmit
# hook entry in settings.json doesn't ENOENT.
cat > "$ACCOUNTS_DIR/auto-rotate-hook.sh" << 'AUTOROTATEHOOK'
#!/usr/bin/env bash
# Auto-rotation hook — DISABLED (no-op).
exit 0
AUTOROTATEHOOK

# Clean up legacy Python artifacts
rm -f "$ACCOUNTS_DIR/rotation-engine.py" 2>/dev/null || true
rm -f "$HOME/.claude/statusline-command.sh" 2>/dev/null || true
rm -f "$HOME/.claude/commands/rotate.md" 2>/dev/null || true

if [[ "$PLATFORM" != "git-bash" ]]; then
    chmod +x "$ACCOUNTS_DIR/statusline-quota.sh" \
             "$ACCOUNTS_DIR/auto-rotate-hook.sh"
    [[ -x "$BIN_DIR/csq" ]] && chmod +x "$BIN_DIR/csq"
fi
ok "Files installed"

# Remove old 'cc' binary if it exists (renamed to csq)
rm -f "$BIN_DIR/cc" 2>/dev/null

# ─── Config dirs ────────────────────────────────────────
for n in 1 2 3 4 5 6 7; do
    mkdir -p "$ACCOUNTS_DIR/config-$n"
done
ok "Config dirs created"

case "$PLATFORM" in
    macos) ok "Credential storage: macOS Keychain + file fallback" ;;
    linux|wsl) ok "Credential storage: file-only (no keychain on Linux)" ;;
    git-bash) ok "Credential storage: file-only (no keychain on Windows)" ;;
esac

# ─── Patch settings.json ────────────────────────────────
SETTINGS_FILE="$HOME/.claude/settings.json"
[[ -f "$SETTINGS_FILE" ]] || echo '{}' > "$SETTINGS_FILE"

# Use jq if available, otherwise basic sed approach
if command -v jq &>/dev/null; then
    desired_sl_cmd='bash ~/.claude/accounts/statusline-quota.sh'
    tmp_settings=$(mktemp)
    jq --arg cmd "$desired_sl_cmd" '.statusLine = {"type":"command","command":$cmd}' "$SETTINGS_FILE" > "$tmp_settings" 2>/dev/null && mv "$tmp_settings" "$SETTINGS_FILE"
    rm -f "$tmp_settings" 2>/dev/null
fi
ok "Settings configured (statusline)"

if ! echo "$PATH" | grep -q "$BIN_DIR"; then
    warn "$BIN_DIR not in PATH. Add to your shell profile:"
    echo "    export PATH=\"$BIN_DIR:\$PATH\""
fi

echo -e "\n${BOLD}Done.${NC} Now save your accounts:\n"
echo "  1. Start Claude:   claude"
echo "  2. Log in:          /login email@example.com"
echo "  3. Save it:         ! csq login 1"
echo "  4. Repeat for each account (slots 1-7)"
echo ""
echo "Daily use:"
echo "  csq run 1           Start CC on account 1 (isolated)"
echo "  csq run 3           Start CC on account 3 (separate terminal)"
echo "  csq status          Show all accounts + quota"
echo ""
echo "When rate limited (inside CC):"
echo "  ! csq swap N         Swap THIS terminal to account N (no restart, same conversation)"
echo "  ! csq suggest        Show which account has the most quota"
echo ""
