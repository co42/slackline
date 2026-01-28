.PHONY: build release test clean

VERSION ?= $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
HOMEBREW_TAP := ../homebrew-slackline
REPO := co42/slackline

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
	@echo "=== Preparing release v$(VERSION) ==="
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
	@echo "=== Waiting for GitHub Actions to build release ==="
	@# Get the run ID for this tag and watch it
	$(eval RUN_ID := $(shell sleep 5 && gh run list -R $(REPO) --branch v$(VERSION) --limit 1 --json databaseId -q '.[0].databaseId'))
	@echo "Watching workflow run $(RUN_ID)..."
	@gh run watch $(RUN_ID) -R $(REPO) --exit-status || (echo "Release build failed!" && exit 1)
	@echo ""
	@echo "=== Updating homebrew formula ==="
	@# Get SHA256 for each platform from release assets
	$(eval SHA_ARM := $(shell gh release view v$(VERSION) -R $(REPO) --json assets -q '.assets[] | select(.name | contains("aarch64-apple-darwin")) | .digest' | sed 's/sha256://'))
	$(eval SHA_X86_MAC := $(shell gh release view v$(VERSION) -R $(REPO) --json assets -q '.assets[] | select(.name | contains("x86_64-apple-darwin")) | .digest' | sed 's/sha256://'))
	$(eval SHA_LINUX := $(shell gh release view v$(VERSION) -R $(REPO) --json assets -q '.assets[] | select(.name | contains("x86_64-unknown-linux-gnu")) | .digest' | sed 's/sha256://'))
	@echo "SHA256 aarch64-apple-darwin: $(SHA_ARM)"
	@echo "SHA256 x86_64-apple-darwin:  $(SHA_X86_MAC)"
	@echo "SHA256 x86_64-unknown-linux: $(SHA_LINUX)"
	@# Update formula
	@sed -i '' 's/version ".*"/version "$(VERSION)"/' $(HOMEBREW_TAP)/Formula/slackline.rb
	@sed -i '' 's|/v[0-9.]*-aarch64-apple-darwin|/v$(VERSION)-aarch64-apple-darwin|g' $(HOMEBREW_TAP)/Formula/slackline.rb
	@sed -i '' 's|/v[0-9.]*-x86_64-apple-darwin|/v$(VERSION)-x86_64-apple-darwin|g' $(HOMEBREW_TAP)/Formula/slackline.rb
	@sed -i '' 's|/v[0-9.]*-x86_64-unknown-linux-gnu|/v$(VERSION)-x86_64-unknown-linux-gnu|g' $(HOMEBREW_TAP)/Formula/slackline.rb
	@sed -i '' 's|download/v[^/]*/slackline|download/v$(VERSION)/slackline|g' $(HOMEBREW_TAP)/Formula/slackline.rb
	@# Update SHAs (in order: arm, x86_mac, linux)
	@awk -v arm="$(SHA_ARM)" -v x86="$(SHA_X86_MAC)" -v linux="$(SHA_LINUX)" \
		'BEGIN{n=0} /sha256/{n++;if(n==1)sub(/sha256 "[^"]*"/,"sha256 \""arm"\"");if(n==2)sub(/sha256 "[^"]*"/,"sha256 \""x86"\"");if(n==3)sub(/sha256 "[^"]*"/,"sha256 \""linux"\"")} {print}' \
		$(HOMEBREW_TAP)/Formula/slackline.rb > $(HOMEBREW_TAP)/Formula/slackline.rb.tmp
	@mv $(HOMEBREW_TAP)/Formula/slackline.rb.tmp $(HOMEBREW_TAP)/Formula/slackline.rb
	@# Commit and push homebrew tap
	@cd $(HOMEBREW_TAP) && git add -A && git commit -m "slackline $(VERSION)" && git push
	@echo ""
	@echo "=== Release v$(VERSION) complete! ==="
	@echo "  - Tagged and pushed slackline"
	@echo "  - GitHub Actions built binaries"
	@echo "  - Updated and pushed homebrew-slackline"
