.PHONY: dev_install install uninstall test build format lint check clean ci-local

CARGO_BIN_DIR := $(HOME)/.cargo/bin
SHELL_RC := $(HOME)/.zshrc
PATH_LINE := export PATH="$(CARGO_BIN_DIR):$$PATH"

# Development install — everything needed to develop with this project
dev_install: install
	rustup component add rustfmt clippy
	./scripts/setup-hooks.sh
	@echo "Development environment ready."

# Runtime install — builds, installs binary, ensures ~/.cargo/bin is on PATH
install:
	@command -v cargo >/dev/null 2>&1 || { echo "Rust not found. Install from https://rustup.rs"; exit 1; }
	cargo build --release
	cargo install --path crates/alfred-bin --force
	@if ! grep -qF '/.cargo/bin' "$(SHELL_RC)" 2>/dev/null; then \
		echo '' >> "$(SHELL_RC)"; \
		echo '# Added by Alfred editor' >> "$(SHELL_RC)"; \
		echo '$(PATH_LINE)' >> "$(SHELL_RC)"; \
		echo "Added $(CARGO_BIN_DIR) to PATH in $(SHELL_RC)"; \
		echo "Run: source $(SHELL_RC)  (or open a new terminal)"; \
	else \
		echo "$(CARGO_BIN_DIR) already on PATH in $(SHELL_RC)"; \
	fi
	@echo "Alfred installed. Run: alfred <file>"

# Uninstall — removes binary and PATH entry
uninstall:
	@rm -f "$(CARGO_BIN_DIR)/alfred"
	@if [ -f "$(SHELL_RC)" ]; then \
		sed -i '' '/# Added by Alfred editor/d' "$(SHELL_RC)"; \
		sed -i '' '\|/.cargo/bin|d' "$(SHELL_RC)"; \
		echo "Removed Alfred from $(CARGO_BIN_DIR) and cleaned $(SHELL_RC)"; \
	else \
		echo "Removed Alfred from $(CARGO_BIN_DIR)"; \
	fi

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

# Run GitHub Actions workflow locally via act (requires Docker)
ci-local:
	act push --container-architecture linux/amd64
