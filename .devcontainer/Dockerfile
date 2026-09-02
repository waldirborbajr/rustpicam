FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

# ==========================================
# 1. Install system dependencies (as root)
# ==========================================
USER root
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y --no-install-recommends \
        ripgrep \
        fd-find \
        bat \
        hyperfine \
        valgrind \
        lldb \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* \
    # Create convenient symlinks for fd and bat (Debian names them differently)
    && ln -s /usr/bin/fdfind /usr/local/bin/fd \
    && ln -s /usr/bin/batcat /usr/local/bin/bat

# ==========================================
# 2. Install Rust tooling (as vscode user)
# ==========================================
USER vscode

# Add core Rust components
RUN rustup component add rustfmt clippy rust-src

# Install cargo-binstall
RUN cargo install --locked cargo-binstall

# Install all available tools via cargo-binstall (pre-built binaries only)
RUN cargo binstall -y --locked \
        cargo-nextest \
        cargo-edit \
        cargo-watch \
        cargo-expand \
        cargo-outdated \
        cargo-audit \
        cargo-udeps \
        cargo-semver-checks \
        cargo-llvm-lines \
        cargo-tarpaulin \
        sccache \
        git-delta \
        just \
        git-lfs

# ==========================================
# 3. Set global environment variables
# ==========================================
# Note: CARGO_INCREMENTAL is removed because sccache handles caching better
ENV RUST_BACKTRACE=1 \
    RUSTFLAGS="-D warnings" \
    CARGO_TERM_COLOR=always \
    RUSTC_WRAPPER=sccache
