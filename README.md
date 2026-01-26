# slack-cli

> **Note**: This project was generated with [Claude Code](https://claude.ai/code).

Read-only Slack CLI for AI agents.

## Daily Summary Workflow

```bash
# Find what needs your attention today
slack-cli search messages 'to:me after:yesterday' -l 30
slack-cli search messages '@yourname after:yesterday' -l 20

# Check messages you sent (for context)
slack-cli search messages 'from:me after:yesterday' -l 20

# Read a specific thread
slack-cli messages replies <CHANNEL_ID> <THREAD_TS>

# Look up who sent a message
slack-cli users info <USER_ID>
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
slack-cli search messages '<query>' -l 20
```

### Channels
```bash
slack-cli me channels                    # Channels you're in
slack-cli channels list -l 50            # All public channels
slack-cli channels history <ID> -l 20    # Read messages
slack-cli channels info <ID>             # Channel details
slack-cli channels members <ID>          # List members
```

### Messages & Threads
```bash
slack-cli messages replies <CH> <TS>     # Read thread
slack-cli messages permalink <CH> <TS>   # Get URL
```

### DMs
```bash
slack-cli dms list                       # List DM conversations
slack-cli dms history <DM_ID> -l 20      # Read DM history
```

### Users
```bash
slack-cli users list -l 50               # List users
slack-cli users info <ID>                # User details
slack-cli users presence <ID>            # Online/away
```

### Auth
```bash
slack-cli auth test                      # Verify token
```

## IDs and Timestamps

- **Channel IDs**: `C...` (e.g., `C1RCG46LS`)
- **DM IDs**: `D...` (e.g., `D032NSG9NAE`)
- **User IDs**: `U...` (e.g., `U032LQBJTH8`)
- **Timestamps**: `1769415774.159039` (for threads/permalinks)

## Output Formats

- Default: Human-readable
- `--json`: Machine-readable JSON
- `-q, --quiet`: Suppress status messages

## Token Setup

Requires `xoxp-...` token with scopes:
- `channels:read`, `channels:history`
- `groups:read`, `groups:history`
- `im:read`, `im:history`
- `mpim:read`, `mpim:history`
- `users:read`
- `search:read`

```bash
export SLACK_TOKEN="xoxp-..."
```

## Example: Daily Catch-up

```bash
# 1. Messages directed to you
slack-cli search messages 'to:me after:yesterday' -l 30

# 2. Mentions in channels
slack-cli search messages '@yourname after:yesterday' -l 20

# 3. Activity in key channels
slack-cli search messages 'in:#infra after:yesterday' -l 15
slack-cli search messages 'in:#team after:yesterday' -l 15

# 4. Read specific thread (from search results)
slack-cli messages replies C03E4DQ9LAJ 1769427215.047649

# 5. Check DMs
slack-cli dms list -l 10
slack-cli dms history D05SGCF75MW -l 10
```
