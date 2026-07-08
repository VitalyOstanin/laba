# 2. Multiple server profiles with a JSON config

Date: 2026-07-08

## Status

Accepted

## Context

The predecessor CLI supported a single OpenProject host: one base URL and one
token. This project must support several OpenProject servers with a selectable
default, and each server may have its own credentials and its own proxy.

## Decision

Store configuration in `$XDG_CONFIG_HOME/taskstream/config.json` as a
map of named server profiles plus a `default_server` field. Each profile holds
`base_url`, `timeout`, `verify_ssl` and an optional `proxy`. Tokens are not kept
in this file (see ADR 0003).

The active server is resolved by precedence: the `--server` flag, then the
`OPENPROJECT_SERVER` environment variable, then `default_server`. A `server`
subcommand manages profiles (list/add/remove/set-default/show).

JSON is used (rather than the predecessor's YAML) as the config format; examples
are documented in the README because JSON has no comments.

## Consequences

- Each server gets an independent HTTP client with its own credentials and
  proxy; nothing leaks between servers.
- The config schema is incompatible with the predecessor's single-host YAML; a
  one-shot `auth import` bridges the two (see ADR 0004).
- The `--server` precedence chain mirrors the token/URL resolution users already
  expect from CLI tools.
