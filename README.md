# slackline

> **Note**: This project was generated with [Claude Code](https://claude.ai/code).

Read-only Slack CLI for AI agents.

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
slackline me channels                    # Channels you're in
slackline channels list -l 50            # All public channels
slackline channels history <ID> -l 20    # Read messages
slackline channels info <ID>             # Channel details
slackline channels members <ID>          # List members
```

### Messages & Threads
```bash
slackline messages replies <CH> <TS>     # Read thread
slackline messages permalink <CH> <TS>   # Get URL
```

### DMs
```bash
slackline dms list                       # List DM conversations
slackline dms history <DM_ID> -l 20      # Read DM history
```

### Users
```bash
slackline users list -l 50               # List users
slackline users info <ID>                # User details
slackline users presence <ID>            # Online/away
```

### Files
```bash
slackline files list                     # List files in workspace
slackline files list -c <CH_ID>          # Files in a channel
slackline files list -u <USER_ID>        # Files by a user
slackline files info <FILE_ID>           # File metadata
slackline files download <FILE_ID> -o f  # Download to file
```

### Token
```bash
slackline token test                     # Verify token works
slackline token create                   # Instructions to create a token
slackline token manifest                 # Print app manifest JSON
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

Required scopes: `channels:read`, `channels:history`, `groups:read`, `groups:history`, `im:read`, `im:history`, `mpim:read`, `mpim:history`, `users:read`, `users:read.email`, `search:read`, `files:read`

```bash
# Set token via environment variable
export SLACK_TOKEN="xoxp-..."

# Or store in macOS Keychain (recommended)
security add-generic-password -s slack-token -a $USER -w 'xoxp-...'
export SLACK_TOKEN=$(security find-generic-password -s slack-token -w)
```

## Example: Daily Catch-up

```bash
# 1. Messages directed to you
slackline search messages 'to:me after:yesterday' -l 30

# 2. Mentions in channels
slackline search messages '@yourname after:yesterday' -l 20

# 3. Activity in key channels
slackline search messages 'in:#infra after:yesterday' -l 15
slackline search messages 'in:#team after:yesterday' -l 15

# 4. Read specific thread (from search results)
slackline messages replies C03E4DQ9LAJ 1769427215.047649

# 5. Check DMs
slackline dms list -l 10
slackline dms history D05SGCF75MW -l 10
```
