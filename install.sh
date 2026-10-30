#!/usr/bin/env bash
set -euo pipefail

# Obsidian Launcher for Noctalia Shell - Installation Script

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
PLUGIN_DIR="${HOME}/.config/noctalia/plugins/obsidian"
HELPER_BIN="${BIN_DIR}/noctalia-obsidian"

echo "==> Installing Noctalia Obsidian Launcher..."

# 1. Check prerequisites
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "${HOME}/.cargo/bin/cargo" ]; then
        export PATH="${HOME}/.cargo/bin:${PATH}"
    else
        echo "Error: cargo (Rust) is not found in PATH. Please install Rust or add cargo to PATH." >&2
        exit 1
    fi
fi

# 2. Build the helper
echo "==> Building noctalia-obsidian (release)..."
(cd "${REPO_DIR}" && cargo build --release)

# 3. Install the helper binary
echo "==> Installing helper binary to ${HELPER_BIN}..."
mkdir -p "${BIN_DIR}"
cp -f "${REPO_DIR}/target/release/noctalia-obsidian" "${HELPER_BIN}"
chmod +x "${HELPER_BIN}"

# 4. First-run config initialization if missing
if [ ! -f "${HOME}/.config/noctalia-obsidian/config.toml" ]; then
    echo "==> Initializing default config at ~/.config/noctalia-obsidian/config.toml..."
    "${HELPER_BIN}" --help >/dev/null 2>&1 || true
fi

# 5. Deploy Noctalia plugin files
echo "==> Deploying Noctalia plugin files to ${PLUGIN_DIR}..."
mkdir -p "${PLUGIN_DIR}"
cp -f "${REPO_DIR}/plugin.toml" "${PLUGIN_DIR}/plugin.toml"
cp -f "${REPO_DIR}/day_launcher.luau" "${PLUGIN_DIR}/day_launcher.luau"
cp -f "${REPO_DIR}/obs_launcher.luau" "${PLUGIN_DIR}/obs_launcher.luau"

# 6. Validate with noctalia plugins lint if noctalia is installed
if command -v noctalia >/dev/null 2>&1; then
    echo "==> Validating plugin..."
    noctalia plugins lint "${PLUGIN_DIR}" || true

    # Enable and reload if daemon is active
    if noctalia msg status >/dev/null 2>&1; then
        echo "==> Enabling plugin and reloading Noctalia config..."
        noctalia msg plugins enable local/obsidian >/dev/null 2>&1 || true
        noctalia msg config-reload >/dev/null 2>&1 || true
    else
        echo "Note: Noctalia daemon is not currently running. It will pick up the plugin on next start."
    fi
fi

echo "==> Installation complete!"
echo "    Helper: ${HELPER_BIN}"
echo "    Config: ${HOME}/.config/noctalia-obsidian/config.toml"
echo "    Plugin: ${PLUGIN_DIR}"
echo "    Usage:  Open launcher and type /day [text] or /obs [query]"
