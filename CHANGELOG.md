# Changelog

All notable changes to this project are recorded here.

## [Unreleased]

## [v0.1.0] - 2026-10-31

### Highlights

First release of an MCP server exposing two tools, `engineering_calculator` and `date_calculator`, each taking a single expression string: the former parses keyboard, typographic, LaTeX and word forms (`degrees`, `choose`, `to hex`) and evaluates with exact whole numbers, decimals to 10 significant digits, number bases and percentages, while the latter handles days between dates, elapsed time, date arithmetic in days, weeks, hours, minutes and seconds, and weekdays, reading ISO, written and offset-bearing dates and refusing ambiguous or unsupported input with a reason. The server runs stateless streamable HTTP at `/mcp` with `/health` and `/admin/version`, refuses requests with disallowed `Host` or a browser `Origin`, and abandons calls after 3 seconds behind the calculators' own 2-second budget. A Dockerfile (non-root, `debian:trixie-slim`, 84 MB for linux/amd64), a compose file and a README covering running, endpoints, configuration and releases are included.

### Features

- The engineering calculator (#2)
- The date calculator (#3)
- The MCP server with its two tools, the image and the README (#4)

### Bug fixes

- The review's findings before the first release (#5)

### Maintenance

- Scaffold the crate, licensing, docs and CI (#1)

### Direct commits

- chore: initial commit (ab30305)
