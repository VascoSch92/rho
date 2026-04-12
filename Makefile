SHELL := /usr/bin/env bash
.SHELLFLAGS := -eu -o pipefail -c

# Colors for output
ECHO := printf '%b\n'
GREEN := \033[32m
YELLOW := \033[33m
RED := \033[31m
CYAN := \033[36m
RESET := \033[0m
UNDERLINE := \033[4m

PREFIX ?= $(HOME)/.local

.PHONY: build check-rust agent-server install uninstall clean help \
       test test-unit test-integration test-snapshots test-snapshots-review

# Default target
.DEFAULT_GOAL := help


check-rust:
	@$(ECHO) "$(YELLOW)Checking Rust toolchain...$(RESET)"
	@if ! command -v cargo &>/dev/null; then \
		$(ECHO) "$(RED)Error: cargo is not installed.$(RESET)"; \
		$(ECHO) "$(YELLOW)Install Rust from https://rustup.rs$(RESET)"; \
		exit 1; \
	fi
	@RUST_VERSION=$$(rustc --version | cut -d' ' -f2); \
	$(ECHO) "$(GREEN)Rust $$RUST_VERSION found.$(RESET)"

agent-server:
	@$(ECHO) "$(YELLOW)Building OpenHands agent server binary...$(RESET)"
	@bash scripts/build-agent-server.sh
	@rm -rf dist/openhands-agent-server
	@mkdir -p dist
	@cp -R scripts/dist/openhands-agent-server dist/openhands-agent-server
	@$(ECHO) "$(GREEN)Binary copied to dist/openhands-agent-server$(RESET)"

build: check-rust agent-server
	@$(ECHO) "$(CYAN)Building Rho...$(RESET)"
	@cargo build
	@$(ECHO) "$(GREEN)Build complete! Development environment is ready.$(RESET)"
	@$(ECHO) ""
	@$(ECHO) "  Run with:  $(CYAN)cargo run$(RESET)"
	@$(ECHO) "  Web mode:  $(CYAN)cargo run -- web$(RESET)"
	@$(ECHO) "  Headless:  $(CYAN)cargo run -- headless --task \"...\"$(RESET)"

install: check-rust agent-server
	@$(ECHO) "$(CYAN)Building Rho (release)...$(RESET)"
	@cargo build --release
	@mkdir -p $(PREFIX)/bin
	@cp target/release/rho $(PREFIX)/bin/rho
	@chmod +x $(PREFIX)/bin/rho
	@rm -rf $(PREFIX)/bin/openhands-agent-server
	@cp -R dist/openhands-agent-server $(PREFIX)/bin/openhands-agent-server
	@$(ECHO) "$(GREEN)Installed to $(PREFIX)/bin/rho$(RESET)"
	@$(ECHO) "$(GREEN)Agent server copied to $(PREFIX)/bin/openhands-agent-server/$(RESET)"
	@if ! echo "$$PATH" | tr ':' '\n' | grep -qx "$(PREFIX)/bin"; then \
		$(ECHO) ""; \
		$(ECHO) "$(YELLOW)WARNING: $(PREFIX)/bin is not in your PATH.$(RESET)"; \
		$(ECHO) "  Add this to your shell profile:"; \
		$(ECHO) "  $(CYAN)export PATH=\"$(PREFIX)/bin:\$$PATH\"$(RESET)"; \
	fi

uninstall:
	@rm -f $(PREFIX)/bin/rho
	@rm -rf $(PREFIX)/bin/openhands-agent-server
	@$(ECHO) "$(GREEN)Uninstalled rho and agent server from $(PREFIX)/bin$(RESET)"

clean:
	@$(ECHO) "$(YELLOW)Cleaning build artifacts...$(RESET)"
	@cargo clean
	@rm -rf scripts/dist scripts/software-agent-sdk
	@$(ECHO) "$(GREEN)Clean complete.$(RESET)"

# ── Tests ────────────────────────────────────────────────────────────

test:
	@$(ECHO) "$(CYAN)Running all tests...$(RESET)"
	@cargo test --test tests
	@$(ECHO) "$(GREEN)All tests passed.$(RESET)"

test-unit:
	@$(ECHO) "$(CYAN)Running unit tests...$(RESET)"
	@cargo test --test tests unit::
	@$(ECHO) "$(GREEN)Unit tests passed.$(RESET)"

test-integration:
	@$(ECHO) "$(CYAN)Running integration tests...$(RESET)"
	@cargo test --test tests integration::
	@$(ECHO) "$(GREEN)Integration tests passed.$(RESET)"

test-snapshots:
	@$(ECHO) "$(CYAN)Running snapshot tests...$(RESET)"
	@cargo test --test tests snapshots::
	@$(ECHO) "$(GREEN)Snapshot tests passed.$(RESET)"

test-snapshots-review:
	@$(ECHO) "$(CYAN)Running snapshot tests and reviewing changes...$(RESET)"
	@cargo insta test --test tests --review
	@$(ECHO) "$(GREEN)Snapshot review complete.$(RESET)"

# Show help
help:
	@$(ECHO) "$(CYAN)Rho Makefile$(RESET)"
	@$(ECHO) ""
	@$(ECHO) "$(UNDERLINE)Usage:$(RESET) make <COMMAND>"
	@$(ECHO) ""
	@$(ECHO) "$(UNDERLINE)Commands:$(RESET)"
	@$(ECHO) "  $(GREEN)build$(RESET)                Check toolchain, build agent server, and compile Rho"
	@$(ECHO) "  $(GREEN)install$(RESET)              Build release binary and install to PREFIX/bin"
	@$(ECHO) "  $(GREEN)uninstall$(RESET)            Remove the installed binary"
	@$(ECHO) "  $(GREEN)agent-server$(RESET)         Build only the OpenHands agent server binary"
	@$(ECHO) "  $(GREEN)clean$(RESET)                Remove all build artifacts"
	@$(ECHO) "  $(GREEN)help$(RESET)                 Show this help message"
	@$(ECHO) ""
	@$(ECHO) "$(UNDERLINE)Testing:$(RESET)"
	@$(ECHO) "  $(GREEN)test$(RESET)                 Run all tests (unit + snapshots + integration)"
	@$(ECHO) "  $(GREEN)test-unit$(RESET)            Run unit tests only"
	@$(ECHO) "  $(GREEN)test-integration$(RESET)     Run integration tests only"
	@$(ECHO) "  $(GREEN)test-snapshots$(RESET)       Run snapshot tests only"
	@$(ECHO) "  $(GREEN)test-snapshots-review$(RESET) Run snapshots and interactively review changes"
	@$(ECHO) ""
	@$(ECHO) "$(UNDERLINE)Options:$(RESET)"
	@$(ECHO) "  $(GREEN)PREFIX$(RESET)         Install prefix (default: ~/.local). Example: make install PREFIX=/usr/local"
