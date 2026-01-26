# slack-cli

A read-only Slack CLI designed for AI agents (Claude) to interact with Slack workspaces.

## Quick Reference

```bash
export SLACK_TOKEN="xoxp-..."  # Set once per session

# List channels and get IDs
slack-cli channels list -l 20

# Read recent messages from a channel
slack-cli channels history <CHANNEL_ID> -l 10

# Read thread replies
slack-cli messages replies <CHANNEL_ID> <THREAD_TS>

# List users
slack-cli users list -l 50

# Get user details
slack-cli users info <USER_ID>
```

## Channel/Message Workflow

1. **Find channel ID**: `slack-cli channels list` → get `id` field (e.g., `C1RCG46LS`)
2. **Read messages**: `slack-cli channels history C1RCG46LS -l 20`
3. **Read thread**: If message has replies, use its `ts` field: `slack-cli messages replies C1RCG46LS 1769415774.159039`

## Commands

| Command | Description | Key Args |
|---------|-------------|----------|
| `auth test` | Verify token works, show workspace info | |
| `channels list` | List channels with IDs | `-l <limit>` |
| `channels info <id>` | Channel details | |
| `channels history <id>` | Recent messages | `-l <limit>` |
| `channels members <id>` | List member user IDs | `-l <limit>` |
| `users list` | List users with IDs | `-l <limit>` |
| `users info <id>` | User details (name, email, title) | |
| `users presence <id>` | Online/away status | |
| `messages replies <ch> <ts>` | Thread replies | `-l <limit>` |
| `messages permalink <ch> <ts>` | Get message URL | |

## Output Formats

- **Default**: Human-readable colored output
- **`--json`**: Machine-readable JSON (better for parsing)
- **`-q, --quiet`**: Suppress status messages

Use `--json` when you need to extract specific fields programmatically.

## IDs and Timestamps

- **Channel IDs**: Start with `C` (e.g., `C1RCG46LS`)
- **User IDs**: Start with `U` (e.g., `U032LQBJTH8`)
- **Message timestamps**: Format `1769415774.159039` (used for threads and permalinks)

## Token

Requires a Slack user token (`xoxp-...`) with these scopes:
- `channels:read`, `channels:history`
- `groups:read`, `groups:history` (private channels)
- `im:read`, `im:history` (DMs)
- `users:read`
- `reactions:read`

Set via `SLACK_TOKEN` env var or `--token` flag.

## Examples

```bash
# Find a channel by browsing the list
slack-cli channels list -l 50 | grep -i random

# Get JSON output for a specific channel's messages
slack-cli --json channels history C1RCG46LS -l 5

# Look up who sent a message
slack-cli users info U03AU4E7DJB

# Check if someone is online
slack-cli users presence U032LQBJTH8
```
