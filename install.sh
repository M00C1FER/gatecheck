#!/usr/bin/env bash
# gatecheck — interactive install wizard.
set -euo pipefail

if [ -t 1 ]; then C_BOLD="$(tput bold)"; C_RESET="$(tput sgr0)"; C_GREEN="$(tput setaf 2)"; C_YELLOW="$(tput setaf 3)"; C_RED="$(tput setaf 1)"; else C_BOLD=""; C_RESET=""; C_GREEN=""; C_YELLOW=""; C_RED=""; fi
say()  { printf "%s%s%s\n" "$C_BOLD" "$1" "$C_RESET"; }
info() { printf "  %s\n" "$1"; }
ok()   { printf "  %s✓%s %s\n" "$C_GREEN" "$C_RESET" "$1"; }
warn() { printf "  %s!%s %s\n" "$C_YELLOW" "$C_RESET" "$1"; }
fail() { printf "  %s✗%s %s\n" "$C_RED" "$C_RESET" "$1" >&2; exit 1; }
prompt_yn() { local q="$1" def="${2:-y}" ans; if [ "$def" = "y" ]; then read -r -p "  $q [Y/n]: " ans; ans="${ans:-y}"; else read -r -p "  $q [y/N]: " ans; ans="${ans:-n}"; fi; [[ "$ans" =~ ^[Yy] ]]; }
prompt_default() { read -r -p "  $1 [$2]: " ans; echo "${ans:-$2}"; }

detect_os() { OS_ID=unknown; OS_LIKE=""; OS_VERSION=""; OS_WSL=0; [ -f /etc/os-release ] && { . /etc/os-release; OS_ID="${ID:-}"; OS_LIKE="${ID_LIKE:-}"; OS_VERSION="${VERSION_ID:-}"; }; [ "$(uname)" = "Darwin" ] && OS_ID=macos; grep -qi microsoft /proc/sys/kernel/osrelease 2>/dev/null && OS_WSL=1 || true; }
pkg_install() {
    case "$OS_ID" in
        debian|ubuntu) sudo apt-get update -qq && sudo apt-get install -y "$@";;
        fedora|rhel|centos) sudo dnf install -y "$@";;
        arch|manjaro) sudo pacman -S --noconfirm "$@";;
        alpine) sudo apk add --no-cache "$@";;
        opensuse*|sles) sudo zypper install -y "$@";;
        macos) brew install "$@";;
        *) warn "unknown OS — install manually: $*"; return 1;;
    esac
}
ensure_rust() {
    command -v cargo >/dev/null && { ok "Rust: $(rustc --version | awk '{print $2}')"; return 0; }
    if prompt_yn "Install Rust via rustup (recommended) ?" y; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
        # shellcheck disable=SC1091
        . "$HOME/.cargo/env" 2>/dev/null || true
    elif prompt_yn "Try system package manager instead?"; then
        case "$OS_ID" in
            debian|ubuntu|fedora|arch|manjaro|alpine|opensuse*|sles|macos) pkg_install rust cargo || pkg_install rustc cargo;;
            *) fail "Rust install failed";;
        esac
    else fail "Rust toolchain required"; fi
}

main() {
    say "gatecheck — install wizard (Rust binary)"
    detect_os
    info "OS: ${OS_ID}${OS_VERSION:+ $OS_VERSION}$([ "$OS_WSL" = 1 ] && echo ' (WSL2)')"

    say ""; say "Step 1/4: Rust toolchain"; ensure_rust

    say ""; say "Step 2/4: Install"
    local BIN_DIR; BIN_DIR="$(prompt_default "Binary directory (must be in \$PATH)" "$HOME/.local/bin")"
    mkdir -p "$BIN_DIR"
    if prompt_yn "Install via 'cargo install' (recommended)?" y; then
        CARGO_INSTALL_ROOT="$HOME/.local" cargo install --git https://github.com/M00C1FER/gatecheck --locked
        # cargo puts it in $CARGO_INSTALL_ROOT/bin
        if [ -x "$HOME/.local/bin/gatecheck" ]; then ok "binary at $HOME/.local/bin/gatecheck"; fi
    else
        local INSTALL_HOME; INSTALL_HOME="$(prompt_default "Source checkout root" "$HOME/.local/share/gatecheck")"
        mkdir -p "$INSTALL_HOME"
        if [ -d "$INSTALL_HOME/.git" ]; then ( cd "$INSTALL_HOME" && git pull -q ); else git clone -q https://github.com/M00C1FER/gatecheck.git "$INSTALL_HOME"; fi
        ( cd "$INSTALL_HOME" && cargo build --release && cp target/release/gatecheck "$BIN_DIR/gatecheck" )
        ok "binary at $BIN_DIR/gatecheck"
    fi

    say ""; say "Step 3/4: Optional config (gatecheck.toml)"
    if prompt_yn "Generate a starter gatecheck.toml in the current directory?" n; then
        local fail_at; fail_at="$(prompt_default "Fail at severity (critical|high|medium|low)" "high")"
        cat > gatecheck.toml <<EOF
fail_at = "$fail_at"
disable = []
exempt_patterns = []
EOF
        ok "wrote ./gatecheck.toml"
    fi

    say ""; say "Step 4/4: Optional pre-commit hook"
    if [ -d .git ] && prompt_yn "Install gatecheck as a pre-commit hook in this repo?" n; then
        cat > .git/hooks/pre-commit <<'EOF'
#!/usr/bin/env bash
set -e
gatecheck --staged --threshold high
EOF
        chmod +x .git/hooks/pre-commit
        ok "wrote .git/hooks/pre-commit"
    fi

    say ""
    ok "Done. Try: gatecheck --list-rules"
}
main "$@"
