.PHONY: build release test clean

VERSION ?= $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
HOMEBREW_TAP := ../homebrew-slackline

build:
	cargo build --release

test:
	cargo test

clean:
	cargo clean

release:
	@if [ -z "$(VERSION)" ]; then echo "Could not determine version"; exit 1; fi
	@if [ ! -d "$(HOMEBREW_TAP)" ]; then echo "Homebrew tap not found at $(HOMEBREW_TAP)"; exit 1; fi
	@if git rev-parse "v$(VERSION)" >/dev/null 2>&1; then echo "Tag v$(VERSION) already exists"; exit 1; fi
	@echo "Preparing release v$(VERSION)..."
	@# Update Cargo.toml version
	@sed -i '' 's/^version = ".*"/version = "$(VERSION)"/' Cargo.toml
	@# Rebuild lock file
	@cargo generate-lockfile
	@# Commit and tag slackline
	@git add -A
	@git commit -m "chore: release v$(VERSION)" || true
	@git tag "v$(VERSION)"
	@git push && git push --tags
	@echo ""
	@echo "Waiting for GitHub to process the tag..."
	@sleep 3
	@# Get SHA256 of the release tarball
	$(eval SHA256 := $(shell curl -sL https://github.com/co42/slackline/archive/refs/tags/v$(VERSION).tar.gz | shasum -a 256 | cut -d' ' -f1))
	@echo "SHA256: $(SHA256)"
	@# Update homebrew tap formula
	@sed -i '' 's|archive/refs/tags/v[^"]*\.tar\.gz|archive/refs/tags/v$(VERSION).tar.gz|' $(HOMEBREW_TAP)/Formula/slackline.rb
	@sed -i '' 's/sha256 ".*"/sha256 "$(SHA256)"/' $(HOMEBREW_TAP)/Formula/slackline.rb
	@# Commit and push homebrew tap
	@cd $(HOMEBREW_TAP) && git add -A && git commit -m "slackline $(VERSION)" && git push
	@echo ""
	@echo "Release v$(VERSION) complete!"
	@echo "  - Tagged and pushed slackline"
	@echo "  - Updated and pushed homebrew-slackline"
