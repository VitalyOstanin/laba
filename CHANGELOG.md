# Changelog

All notable changes to this project are documented here.
The format follows Keep a Changelog and Semantic Versioning.

## [Unreleased]

### Added
- Cargo workspace: `core` library and `taskstream` CLI.
- Server profiles (JSON config) with default selection; SOCKS5/HTTP proxy per server.
- Token storage in the system keyring with a file fallback.
- `auth` (login/status/token/logout/import) and `server` (list/add/remove/set-default/show) commands.
