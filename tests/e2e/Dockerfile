# Dockerfile for Alfred end-to-end tests.
#
# Single-stage build: compiles Alfred and runs the Python test suite
# in the same image to avoid glibc version mismatches between builder
# and runtime.

FROM rust:latest

WORKDIR /alfred

# Install Python + pexpect + pytest
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        python3 python3-pip python3-venv \
        procps && \
    rm -rf /var/lib/apt/lists/*

RUN python3 -m venv /opt/venv && \
    /opt/venv/bin/pip install --no-cache-dir pexpect pytest

# Copy manifests first to cache dependency resolution
COPY Cargo.toml Cargo.lock ./
COPY crates/alfred-core/Cargo.toml crates/alfred-core/Cargo.toml
COPY crates/alfred-lisp/Cargo.toml crates/alfred-lisp/Cargo.toml
COPY crates/alfred-plugin/Cargo.toml crates/alfred-plugin/Cargo.toml
COPY crates/alfred-tui/Cargo.toml crates/alfred-tui/Cargo.toml
COPY crates/alfred-bin/Cargo.toml crates/alfred-bin/Cargo.toml

# Create stub lib/main files so cargo can resolve the workspace
RUN mkdir -p crates/alfred-core/src && echo "" > crates/alfred-core/src/lib.rs && \
    mkdir -p crates/alfred-lisp/src && echo "" > crates/alfred-lisp/src/lib.rs && \
    mkdir -p crates/alfred-plugin/src && echo "" > crates/alfred-plugin/src/lib.rs && \
    mkdir -p crates/alfred-tui/src && echo "" > crates/alfred-tui/src/lib.rs && \
    mkdir -p crates/alfred-bin/src && echo "fn main() {}" > crates/alfred-bin/src/main.rs

# Pre-fetch dependencies (cached unless Cargo.toml/Cargo.lock change)
RUN cargo fetch

# Now copy the real source code
COPY crates/ crates/

# Touch source files to invalidate the stub build cache
RUN find crates -name "*.rs" -exec touch {} +

# Build in release mode
RUN cargo build --release --bin alfred

# Install the binary
RUN cp target/release/alfred /usr/local/bin/alfred

# Copy plugins (Alfred loads them from ./plugins/ relative to CWD)
COPY plugins/ /alfred/plugins/

# Copy the test driver
COPY tests/e2e/test_alfred.py /alfred/tests/e2e/test_alfred.py

# Use the venv Python
ENV PATH="/opt/venv/bin:$PATH"
ENV TERM=xterm-256color

CMD ["python3", "-m", "pytest", "-v", "/alfred/tests/e2e/test_alfred.py"]
