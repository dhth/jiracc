# jiracc

`jiracc` synchronizes Jira issues into a local snapshot for offline search and
inspection. It only supports on-premise Jira installations.

This project uses `mise` for tool management and tasks. Always use `mise` to
execute project commands; see `mise.toml` for the available tasks.

```text
configuration → Jira API → snapshot storage → search/show
```

- `cli` defines the command-line interface and translates CLI arguments into
  application commands.
- `application` contains command orchestration and the config, sync, search,
  and show use cases.
- `domain` contains the core issue and snapshot models.
- `jira` contains the Jira API client and transport types.
- `persistence` contains local snapshot storage implementations.
- `config` and `paths` handle configuration and platform-specific filesystem
  paths.

Keep Jira transport concerns inside `jira` and convert them to domain types at
that boundary. Keep orchestration in `application`, not in the CLI. `search` and
`show` operate only on persisted snapshots; only `sync` communicates with Jira.

Prefer pure transformations and explicit inputs and outputs while keeping the
code idiomatic to Rust. Keep filesystem, network, environment, and terminal
effects at application or adapter boundaries. Use local mutation when it makes
the code clearer.

Prefer integration tests for user-visible CLI behavior, following the existing
fixture patterns, and use snapshot testing where relevant. Use `mise run
update-snapshots` to update snapshots; `mise run review-snapshots` is reserved
for human review.
