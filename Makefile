.PHONY: dev_install install test build format lint check clean

# Development install — everything needed to develop with this project
dev_install: install
	rustup component add rustfmt clippy
	./scripts/setup-hooks.sh
	@echo "Development environment ready."

# Runtime install — everything needed to run Alfred
install:
	@command -v cargo >/dev/null 2>&1 || { echo "Rust not found. Install from https://rustup.rs"; exit 1; }
	cargo build --release
	cargo install --path crates/alfred-bin --force
	@echo "Alfred installed. Run: alfred <file>"

# Run all tests
test:
	cargo test --workspace

# Build the project (debug)
build:
	cargo build --workspace

# Format the code
format:
	cargo fmt --all -- --check

# Lint the code
lint:
	cargo clippy --workspace -- -D warnings

# Compile check (fast, no codegen)
check:
	cargo check --workspace

# Clean build artifacts
clean:
	cargo clean
