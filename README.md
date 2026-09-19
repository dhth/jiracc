<p align="center">
  <h1 align="center">jiracc</h1>
  <p align="center">
    <a href="https://github.com/dhth/jiracc/actions/workflows/main.yml"><img alt="Build" src="https://img.shields.io/github/actions/workflow/status/dhth/jiracc/main.yml?style=flat-square"></a>
  </p>
</p>

`jiracc` lets you access your JIRA issues offline.

> [!WARNING]
> jiracc is alpha software. Its interface and behavior might change in the near
> future.

🤔 Motivation
---

The Jira Data Center deployment at my day job is only accessible from a private
network. Connecting to a VPN whenever I need to look up an issue is tedious.
`jiracc` lets me synchronize the issues I care about while connected and access
them locally afterward.

> [!NOTE]
> `jiracc` supports Jira Data Center only. Jira Cloud is not supported.

💾 Installation
---

Install `jiracc` from source using the Rust toolchain:

```sh
cargo install --git https://github.com/dhth/jiracc
```

⚡️ Getting started
---

Create a configuration file at the default path:

```sh
jiracc config init
```

This creates a configuration file with the following contents:

```toml
# Configuration values can reference environment variables. Referenced
# variables need to be set before running jiracc.

[jira]
# Base URL of your Jira Data Center deployment.
url = "https://jira.example.com"

# Personal access token used to authenticate with Jira.
token = "$JIRACC_JIRA_TOKEN"

# JQL that defines the issues jiracc synchronizes.
jql = """
project = EXAMPLE
ORDER BY updated DESC
"""
```

Edit the file with your Jira URL and the JQL that selects the issues to
synchronize. Then set your personal access token, validate the configuration,
and check that Jira accepts the token:

```sh
export JIRACC_JIRA_TOKEN="your-personal-access-token"
jiracc config validate
jiracc auth check
```

🔄 Workflow
---

First, synchronize the issues selected by the JQL in your configuration:

```sh
jiracc sync
```

This saves a local snapshot. You can then search the snapshot without a network
connection or Jira credentials:

### Search issues

```sh
jiracc search "rescue"
jiracc search --assignee leon.kennedy --status "In Progress" --type Task
```

Search results are printed as a compact table:

```text
KEY     TYPE   STATUS       ASSIGNEE          SUMMARY
OPS-42  Task   In Progress  @leon.kennedy     Rescue the president's daughter
LAB-51  Task   To Do        @luis.sera        Retrieve suppressants from the lab
ORG-58  Story  In Progress  @ada.wong         Secure the Amber
PLG-71  Task   To Do        @ramon.salazar    Keep intruders out of the castle
PLG-84  Story  In Progress  @jack.krauser     Settle unfinished business with Leon
PLG-90  Epic   In Progress  @osmund.saddler   Spread Las Plagas beyond the island
```

A text query matches issue keys, summaries, and descriptions. Query and filter
values use case-insensitive substring matching. Filters can be repeated; values
within one filter are combined with OR, while different filters are combined
with AND.

### Show an issue

You can view the cached details for an issue via the `show` command:

```sh
jiracc show OPS-42
```

```text
OPS-42 — Rescue the president's daughter

Type:       Task
Status:     In Progress
Assignee:   Leon S. Kennedy (@leon.kennedy)
Updated:    2004-09-15T10:00:00.000+0000
Jira ID:    10042
Parent:     OPS-40
Subtasks:   OPS-43, OPS-44

Description
-----------
Locate Ashley Graham, get her out of the village, and bring her home safely.
Along the way, execute as many roundhouse kicks and deliver as many one-liners
as possible. Keep the attaché case organized and do not ask the merchant where
he gets his inventory.
```

Run `jiracc sync` again whenever you want to replace the snapshot with current
data from Jira.

`>_` Commands
---

| Command                  | What it does                                      |
|--------------------------|---------------------------------------------------|
| `jiracc config init`     | Create a sample configuration at the default path |
| `jiracc config sample`   | Print a sample configuration                      |
| `jiracc config validate` | Validate the configuration                        |
| `jiracc auth check`      | Check the configured Jira credentials             |
| `jiracc sync`            | Synchronize issues from Jira                      |
| `jiracc status`          | Show information about the local issue cache      |
| `jiracc search [QUERY]`  | Search or list cached issues                      |
| `jiracc show ISSUE-KEY`  | Show the cached details of an issue               |
| `jiracc help`            | Show all commands                                 |

`auth check` and `sync` communicate with Jira. `status`, `search`, and `show`
only read the local issue cache. Commands that use a configuration file accept
`--config-path` to use a file other than the default. Run
`jiracc <command> --help` for command options and supported flags.
