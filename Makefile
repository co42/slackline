.PHONY: build release test clean

VERSION ?= $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')

build:
	cargo build --release

test:
	cargo test

clean:
	cargo clean

release:
	@if [ -z "$(VERSION)" ]; then echo "Could not determine version"; exit 1; fi
	@echo "Preparing release v$(VERSION)..."
	@# Update Cargo.toml version
	@sed -i '' 's/^version = ".*"/version = "$(VERSION)"/' Cargo.toml
	@# Update Formula URL and reset SHA
	@sed -i '' 's|archive/refs/tags/v[^"]*\.tar\.gz|archive/refs/tags/v$(VERSION).tar.gz|' Formula/slackline.rb
	@sed -i '' 's/sha256 ".*"/sha256 "PLACEHOLDER_SHA256"/' Formula/slackline.rb
	@# Rebuild lock file
	@cargo generate-lockfile
	@echo ""
	@echo "Updated to v$(VERSION). Review changes, then:"
	@echo "  git add -A && git commit -m 'chore: release v$(VERSION)'"
	@echo "  git tag v$(VERSION)"
	@echo "  git push && git push --tags"
	@echo ""
	@echo "After release is published, update SHA256:"
	@echo "  curl -sL https://github.com/co42/slackline/archive/refs/tags/v$(VERSION).tar.gz | shasum -a 256"
