# Tech Landscape Research — csq v2.0

## Recommended Stack

| Component    | Technology                                  | Rationale                                                        |
| ------------ | ------------------------------------------- | ---------------------------------------------------------------- |
| Framework    | Tauri v2                                    | Production-ready, 10MB bundle (vs 300MB Electron)                |
| Frontend     | Svelte 4/5                                  | Smaller bundle than React, compiles to vanilla JS                |
| OAuth        | `oauth2` v5.0 crate                         | Built-in PKCE, type-safe                                         |
| Credentials  | `keyring` v4 crate                          | macOS Keychain, Linux Secret Service, Windows Credential Manager |
| File I/O     | `tempfile` + `std::fs::rename`              | Atomic writes for credential safety                              |
| Async        | Tokio 1.48 (managed by Tauri)               | Multi-threaded, no extra init                                    |
| HTTP         | `reqwest` (Tauri plugin)                    | Async, integrated                                                |
| Process mgmt | `std::process::Command`                     | Cross-platform claude CLI spawning                               |
| Distribution | NSIS (Win) + DMG (macOS) + AppImage (Linux) | Built-in to Tauri                                                |
| Auto-update  | Tauri updater plugin                        | Ed25519 signed, mandatory                                        |
| CI/CD        | GitHub Actions + tauri-action               | Cross-platform builds                                            |

## Competing Products

| Product                    | Architecture            | Multi-account OAuth | Quota rotation | Gap vs csq                          |
| -------------------------- | ----------------------- | :-----------------: | :------------: | ----------------------------------- |
| **Opcode**                 | Tauri+React, ~19k stars |         No          |       No       | Visual browser, no rotation         |
| **Nimbalyst**              | Desktop app             |         No          |       No       | Workflow-oriented, no accounts      |
| **CCManager**              | Multi-IDE               |       Partial       |       No       | Broad IDE support, shallow features |
| **Usage4Claude**           | macOS menu bar          |         No          |       No       | Monitor only                        |
| **Claude Squad (smtg-ai)** | tmux TUI                |         No          |       No       | TUI, no OAuth                       |

**csq's unique position**: Only tool combining multi-account OAuth + quota-aware rotation + per-terminal isolation + provider pooling + desktop dashboard. No competitor addresses this combination.

## Bundle Sizes (Tauri v2)

| Platform            | Minimal  | Typical   |
| ------------------- | -------- | --------- |
| macOS ARM64 (.app)  | 20-30 MB | 40-80 MB  |
| Windows NSIS (.exe) | 10-20 MB | 30-60 MB  |
| Linux AppImage      | 30-50 MB | 60-120 MB |
| Linux .deb          | 5-10 MB  | 20-40 MB  |

## Key Tauri Plugins

- `tauri-plugin-single-instance` — prevent multiple daemons
- `tauri-plugin-autostart` — boot-time auto-launch
- `tauri-plugin-stronghold` — encrypted credential storage (IOTA)
- `tauri-plugin-updater` — Ed25519 auto-update
- System tray — built-in, no plugin needed

## Critical Technical Details

- Refresh tokens are single-use and rotate on every exchange
- Access tokens expire in 60 minutes
- Tauri manages tokio runtime — no manual setup needed
- Svelte compiles to vanilla JS (30-50KB vs React's 40-100KB)
- PKCE required for OAuth flow
