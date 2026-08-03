# syntax=docker/dockerfile:1

# -----------------------------------------------------------------------------
# Build stage — cross-compile nativo via Buildx (sem QEMU lento)
# -----------------------------------------------------------------------------
FROM --platform=$BUILDPLATFORM rust:1.85-bookworm AS builder

ARG TARGETPLATFORM
ARG BUILDPLATFORM

WORKDIR /src

# Mapeia plataforma Docker → target triple Rust
RUN case "$TARGETPLATFORM" in \
      "linux/arm64")  echo "aarch64-unknown-linux-gnu" > /tmp/rust-target ;; \
      "linux/amd64")  echo "x86_64-unknown-linux-gnu"  > /tmp/rust-target ;; \
      *) echo "unsupported platform: $TARGETPLATFORM" >&2; exit 1 ;; \
    esac

RUN rustup target add "$(cat /tmp/rust-target)"

# Cross-compiler só quando o runner é amd64 e o alvo é arm64
RUN if [ "$TARGETPLATFORM" = "linux/arm64" ] && [ "$BUILDPLATFORM" = "linux/amd64" ]; then \
      dpkg --add-architecture arm64 && \
      apt-get update && \
      apt-get install -y --no-install-recommends \
        gcc-aarch64-linux-gnu \
        libc6-dev-arm64-cross; \
    fi

# Cache de dependências
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN RUST_TARGET=$(cat /tmp/rust-target) && \
    if [ "$TARGETPLATFORM" = "linux/arm64" ] && [ "$BUILDPLATFORM" = "linux/amd64" ]; then \
      export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc; \
    fi && \
    cargo build --release --target "$RUST_TARGET" || true

# Código real
COPY src ./src
RUN touch src/main.rs && \
    RUST_TARGET=$(cat /tmp/rust-target) && \
    if [ "$TARGETPLATFORM" = "linux/arm64" ] && [ "$BUILDPLATFORM" = "linux/amd64" ]; then \
      export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc; \
    fi && \
    cargo build --release --target "$RUST_TARGET" && \
    cp "target/$RUST_TARGET/release/phone-cam-telegram" /phone-cam-telegram

# -----------------------------------------------------------------------------
# Runtime — Debian slim com adb + scrcpy + ffmpeg
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
      adb \
      ffmpeg \
      scrcpy \
      ca-certificates \
      libsdl2-2.0-0 \
    && rm -rf /var/lib/apt/lists/* \
    && scrcpy --version \
    && ffmpeg -version | head -1 \
    && adb version

COPY --from=builder /phonecam /usr/local/bin/phonecam

# Usuário não-root (mesmo UID do exemplo Go)
RUN useradd -u 65532 -m appuser
USER 65532

ENTRYPOINT ["/usr/local/bin/phonecam"]
# Exemplo:
#   command: ["--facing", "back"]
#   environment:
#     TELEGRAM_BOT_TOKEN: ...
#     TELEGRAM_CHAT_ID: ...
