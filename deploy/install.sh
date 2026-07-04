#!/usr/bin/env bash
# Install script for copied (clipboard manager for Pop!_OS 24.04 / COSMIC / Wayland).
#
# What this does, in order:
#   1. Builds and installs the daemon + TUI client binaries via `cargo install`.
#   2. Writes /etc/profile.d/copied-clipboard.sh to enable the Wayland data-control
#      protocol system-wide (requires sudo + a REBOOT to take effect).
#   3. Installs and enables the copied-daemon systemd user unit.
#   4. Prints instructions for the COSMIC keyboard shortcut and how to verify.
#
# Safe to re-run. Every step that needs sudo prints the exact command first and
# asks for confirmation before running it. No destructive operations.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
PROFILE_D_FILE="/etc/profile.d/copied-clipboard.sh"

confirm() {
    local prompt="$1"
    read -r -p "$prompt [y/N] " reply
    case "$reply" in
        [yY][eE][sS]|[yY]) return 0 ;;
        *) return 1 ;;
    esac
}

echo "==> Step 1/4: Build and install binaries (cargo install)"
if command -v cargo >/dev/null 2>&1; then
    (
        cd "$REPO_ROOT"
        cargo install --path crates/copied-daemon --force
        cargo install --path crates/copied-cli --force
    )
    echo "Installed: $HOME/.cargo/bin/copied-daemon and $HOME/.cargo/bin/copied"
else
    cat <<'EOF'
cargo not found. Build manually and copy the binaries yourself, e.g.:

    cargo build --release --manifest-path crates/copied-daemon/Cargo.toml
    cargo build --release --manifest-path crates/copied-cli/Cargo.toml
    mkdir -p ~/.cargo/bin
    cp target/release/copied-daemon ~/.cargo/bin/
    cp target/release/copied ~/.cargo/bin/

Skipping automatic build.
EOF
fi

echo
echo "==> Step 2/4: Enable Wayland clipboard data-control protocol system-wide"
echo "This writes the following file (requires sudo):"
echo
echo "  $PROFILE_D_FILE"
echo '  ---'
echo '  export COSMIC_DATA_CONTROL_ENABLED=1'
echo '  ---'
echo
if [ -f "$PROFILE_D_FILE" ] && grep -q "COSMIC_DATA_CONTROL_ENABLED=1" "$PROFILE_D_FILE" 2>/dev/null; then
    echo "Already present, skipping."
else
    if confirm "Run 'sudo tee $PROFILE_D_FILE' and 'sudo chmod 644 $PROFILE_D_FILE' now?"; then
        echo 'export COSMIC_DATA_CONTROL_ENABLED=1' | sudo tee "$PROFILE_D_FILE" >/dev/null
        sudo chmod 644 "$PROFILE_D_FILE"
        echo "Written and chmod'd."
    else
        echo "Skipped. You can create it manually later:"
        echo "  echo 'export COSMIC_DATA_CONTROL_ENABLED=1' | sudo tee $PROFILE_D_FILE"
        echo "  sudo chmod 644 $PROFILE_D_FILE"
    fi
fi
echo
echo "IMPORTANT: this env var is read by the COSMIC compositor at session start."
echo "It has NO effect until you REBOOT (logging out is not enough, since the"
echo "compositor itself needs to restart with the variable set)."

echo
echo "==> Step 3/4: Install systemd user unit"
mkdir -p "$SYSTEMD_USER_DIR"
cp "$REPO_ROOT/deploy/copied-daemon.service" "$SYSTEMD_USER_DIR/copied-daemon.service"
echo "Copied to $SYSTEMD_USER_DIR/copied-daemon.service"
systemctl --user daemon-reload
systemctl --user enable --now copied-daemon.service
echo "Enabled and started copied-daemon.service (user unit)."

echo
echo "==> Step 4/4: Keyboard shortcut (manual step)"
cat <<'EOF'
Configure a custom keyboard shortcut in:

    COSMIC Settings > Keyboard > Custom Shortcuts

Command to bind (adjust terminal if you don't use cosmic-term):

    cosmic-term -e copied

EOF

echo "==> Done. Verify with:"
echo
echo "    systemctl --user status copied-daemon.service"
echo
echo "If you skipped or just created the profile.d file, REBOOT before"
echo "expecting the daemon to see clipboard events."
