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

.PHONY: build check-rust agent-server clean help

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
	@mkdir -p dist
	@cp scripts/dist/openhands-agent-server dist/openhands-agent-server
	@$(ECHO) "$(GREEN)Binary copied to dist/openhands-agent-server$(RESET)"

build: check-rust agent-server
	@$(ECHO) "$(CYAN)Building Rho...$(RESET)"
	@cargo build
	@$(ECHO) "$(GREEN)Build complete! Development environment is ready.$(RESET)"
	@$(ECHO) ""
	@$(ECHO) "  Run with:  $(CYAN)cargo run$(RESET)"
	@$(ECHO) "  Web mode:  $(CYAN)cargo run -- web$(RESET)"
	@$(ECHO) "  Headless:  $(CYAN)cargo run -- headless --task \"...\"$(RESET)"

clean:
	@$(ECHO) "$(YELLOW)Cleaning build artifacts...$(RESET)"
	@cargo clean
	@rm -rf scripts/dist scripts/software-agent-sdk
	@$(ECHO) "$(GREEN)Clean complete.$(RESET)"

# Show help
help:
	@$(ECHO) "$(CYAN)Rho Makefile$(RESET)"
	@$(ECHO) ""
	@$(ECHO) "$(UNDERLINE)Usage:$(RESET) make <COMMAND>"
	@$(ECHO) ""
	@$(ECHO) "$(UNDERLINE)Commands:$(RESET)"
	@$(ECHO) "  $(GREEN)build$(RESET)          Check toolchain, build agent server binary, and compile Rho"
	@$(ECHO) "  $(GREEN)agent-server$(RESET)   Build only the OpenHands agent server binary into dist/"
	@$(ECHO) "  $(GREEN)clean$(RESET)          Remove all build artifacts"
	@$(ECHO) "  $(GREEN)help$(RESET)           Show this help message"
