.PHONY: build serve clean prebuild

NIX = nix develop --command

# Build the site: run pre-build math script, then zola build
build: prebuild
	$(NIX) zola build

# Serve the site locally with live reload.
# A watchexec process re-runs the prebuild whenever src/ changes; zola serve
# watches content/ and rebuilds from the regenerated files. Without this,
# editing src/ during `make serve` would show no change (zola does not watch
# src/, and it cannot run the prebuild itself). The EXIT trap tears down
# watchexec when zola exits (including on Ctrl-C); errors are suppressed in
# case watchexec already died.
serve: prebuild
	@$(NIX) sh -c 'trap "kill $$! 2>/dev/null || true" EXIT; \
		watchexec -w src --postpone --debounce 500 --shell none -- cargo run --manifest-path scripts/prebuild/Cargo.toml & \
		zola serve'

# Run the pre-build math rendering script
prebuild:
	$(NIX) cargo run --manifest-path scripts/prebuild/Cargo.toml

# Clean build artifacts
clean:
	rm -rf public content
