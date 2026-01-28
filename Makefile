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
	@echo "Preparing release v$(VERSION)..."
	@# Update Cargo.toml version
	@sed -i '' 's/^version = ".*"/version = "$(VERSION)"/' Cargo.toml
	@# Rebuild lock file
	@cargo generate-lockfile
	@# Update homebrew tap formula
	@sed -i '' 's|archive/refs/tags/v[^"]*\.tar\.gz|archive/refs/tags/v$(VERSION).tar.gz|' $(HOMEBREW_TAP)/Formula/slackline.rb
	@sed -i '' 's/sha256 ".*"/sha256 "PLACEHOLDER_SHA256"/' $(HOMEBREW_TAP)/Formula/slackline.rb
	@echo ""
	@echo "Updated to v$(VERSION). Review changes in both repos, then:"
	@echo ""
	@echo "  # In slackline:"
	@echo "  git add -A && git commit -m 'chore: release v$(VERSION)'"
	@echo "  git tag v$(VERSION)"
	@echo "  git push && git push --tags"
	@echo ""
	@echo "  # After release is published, update homebrew-slackline SHA256:"
	@echo "  curl -sL https://github.com/co42/slackline/archive/refs/tags/v$(VERSION).tar.gz | shasum -a 256"
	@echo ""
	@echo "  # In homebrew-slackline:"
	@echo "  # Update sha256 in Formula/slackline.rb with the value above"
	@echo "  git add -A && git commit -m 'slackline $(VERSION)'"
	@echo "  git push"
