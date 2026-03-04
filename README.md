# slackline

Slack CLI.

## Install

```bash
# Homebrew (macOS)
brew install co42/slackline/slackline

# Cargo
cargo install --git https://github.com/co42/slackline

# Binary releases (macOS ARM, macOS Intel, Linux)
# https://github.com/co42/slackline/releases
```

## Search Syntax (Most Useful)

| Query | Description |
|-------|-------------|
| `to:me` | Messages sent to you |
| `to:me after:yesterday` | Messages to you since yesterday |
| `from:@username` | Messages from a specific user |
| `@username` | Mentions of a user |
| `in:#channel keyword` | Keyword in specific channel |
| `in:#channel after:yesterday` | Recent activity in channel |
| `has:link` | Messages containing links |
| `has:reaction` | Messages with reactions |
| `before:today` | Messages before today |

## Commands Reference

### Search (primary tool for agents)
```bash
slackline search messages '<query>' -l 20
```

### Channels
```bash
slackline me channels                                      # Channels you're in
slackline me channels --unread                             # Channels with unread messages
slackline me channels --unread --dms                       # Include DMs with unreads
slackline channels list -l 50                              # All public channels
slackline channels history <ID> -l 20                      # Read messages
slackline channels info <ID>                               # Channel details
slackline channels members <ID>                            # List members
slackline channels pins <ID>                               # List pinned messages
```

### Messages & Threads
```bash
slackline messages replies <CH> <TS>                       # Read thread
slackline messages permalink <CH> <TS>                     # Get URL
slackline messages reactions <CH> <TS>                     # Get reactions
slackline messages send <CH> "text"                        # Send a message
slackline messages send <CH> "text" --thread-ts <TS>       # Reply in thread
slackline messages react <CH> <TS> thumbsup                # Add reaction
slackline messages unreact <CH> <TS> thumbsup              # Remove reaction
slackline messages pin <CH> <TS>                           # Pin a message
slackline messages unpin <CH> <TS>                         # Unpin a message
```

### DMs
```bash
slackline dms list                                         # List DM conversations
slackline dms history <DM_ID> -l 20                        # Read DM history
slackline dms send <USER_ID> "text"                        # Send a DM
```

### Users
```bash
slackline users list -l 50                                 # List users
slackline users search "peter"                             # Search by name/email
slackline users info <ID>                                  # User details
slackline users presence <ID>                              # Online/away
```

### Files
```bash
slackline files list                                       # List files in workspace
slackline files list -c <CH_ID>                            # Files in a channel
slackline files list -u <USER_ID>                          # Files by a user
slackline files info <FILE_ID>                             # File metadata
slackline files download <FILE_ID> -o f                    # Download to file
slackline files upload ./report.pdf -c <CH_ID>             # Upload to channel
slackline files upload ./img.png -c <CH_ID> --comment "…"  # Upload with comment
```

### Status
```bash
slackline me set-status "In a meeting" -e ":calendar:"     # Set status
slackline me clear-status                                  # Clear status
```

### Watch (Socket Mode event streaming)
```bash
slackline watch                                            # Stream all events (default: message,dm,reaction)
slackline watch --events all                               # Stream all event types
slackline watch --events message,reaction                  # Only messages and reactions
slackline watch --channels C1RCG46LS,C0AB2G3EY             # Filter to specific channels
slackline watch --raw                                      # Output raw slack-morphism event JSON
```

Requires `SLACK_TOKEN` (xoxp-...) and `SLACK_APP_TOKEN` (xapp-...). Events stream as JSONL to stdout. Uses your user token so you receive events from all channels you're a member of — no bot invite needed.

**Quick setup:**
```bash
slackline token create --watch                             # Opens Slack with pre-configured manifest
# Follow the steps, then:
export SLACK_TOKEN='xoxp-...'                              # User token
export SLACK_APP_TOKEN='xapp-...'                          # App-level token (Socket Mode)
slackline watch
```

### Token
```bash
slackline token test                                       # Verify token works
slackline token create                                     # Create read-only app
slackline token create --write                             # Include write scopes
slackline token create --watch                             # Include Socket Mode for watch
slackline token create --write --watch                     # Write + watch
slackline token manifest                                   # Print read-only manifest
slackline token manifest --write --watch                   # Print full manifest
```

## Read-only Mode

Set `SLACKLINE_READONLY` to disable all write operations. Write commands are hidden from help and return an error if invoked directly.

```bash
export SLACKLINE_READONLY=1
```

## IDs and Timestamps

- **Channel IDs**: `C...` (e.g., `C1RCG46LS`)
- **DM IDs**: `D...` (e.g., `D032NSG9NAE`)
- **User IDs**: `U...` (e.g., `U032LQBJTH8`)
- **File IDs**: `F...` (e.g., `F0AB1G1EY5V`)
- **Timestamps**: `1769415774.159039` (for threads/permalinks)

## Output Formats

- Default: Human-readable
- `--json`: Machine-readable JSON
- `-q, --quiet`: Suppress status messages

## Token Setup

The easiest way to create a token:

```bash
slackline token create
```

This prints a URL that opens Slack's app creation page with all required scopes pre-configured. Follow the steps to install the app and copy your token.

Read scopes: `channels:read`, `channels:history`, `groups:read`, `groups:history`, `im:read`, `im:history`, `mpim:read`, `mpim:history`, `users:read`, `users:read.email`, `search:read`, `files:read`, `pins:read`, `reactions:read`

Write scopes: `chat:write`, `files:write`, `im:write`, `pins:write`, `reactions:write`, `users.profile:write`

```bash
# Set token via environment variable
export SLACK_TOKEN="xoxp-..."

# Or store in macOS Keychain (recommended)
security add-generic-password -s slack-token -a $USER -w 'xoxp-...'
export SLACK_TOKEN=$(security find-generic-password -s slack-token -w)
```

## Development

```bash
# Enable pre-commit hooks (fmt + clippy)
git config core.hooksPath .githooks
```

## Example: Daily Catch-up

```bash
# 1. Channels with unread messages
slackline me channels --unread

# 2. Messages directed to you
slackline search messages 'to:me after:yesterday' -l 30

# 3. Mentions in channels
slackline search messages '@yourname after:yesterday' -l 20

# 4. Activity in key channels
slackline search messages 'in:#infra after:yesterday' -l 15
slackline search messages 'in:#team after:yesterday' -l 15

# 5. Read specific thread (from search results)
slackline messages replies C03E4DQ9LAJ 1769427215.047649

# 6. Check DMs
slackline dms list -l 10
slackline dms history D05SGCF75MW -l 10
```
