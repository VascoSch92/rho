.PHONY: build clean check-uv

VENV := .venv
UV := uv

check-uv:
	@which $(UV) > /dev/null 2>&1 || (echo "Error: '$(UV)' is not installed. Install it from https://docs.astral.sh/uv/getting-started/installation/" && exit 1)

build: check-uv clean $(VENV)/.installed

$(VENV)/.installed: pyproject.toml
	@echo "Installing dependencies..."
	$(UV) sync --python 3.12
	@echo "Done."
	@touch $@

clean:
	rm -rf $(VENV)
