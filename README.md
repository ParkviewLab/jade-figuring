<!--
SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
SPDX-License-Identifier: CC-BY-4.0
-->

# jade-figuring

A calculator MCP server for language models: an engineering calculator and date arithmetic, offered as two tools with definitions small enough for a model with a short context. A model passes an ordinary expression, written as it would write one, and receives the result with the expression as it was read:

| Tool | The model sends | It receives |
|---|---|---|
| `engineering_calculator` | `sin(30 degrees)` | `sin(30°) = 0.5` |
| | `\frac{52!}{5! \times (52-5)!}` | `52! / (5! × (52 - 5)!) = 2598960` |
| | `0xFF3A to binary` | `0xFF3A to binary = 0b1111111100111010` |
| | `100 + 3%` | `100 + 3% = 103` |
| `date_calculator` | `days between 2026-10-31 and 2026-12-25` | `from 2026-10-31 to 2026-12-25 = 55 days (7 weeks and 6 days)` |
| | `weekday of July 4, 2027` | `weekday of 2027-07-04 = Sunday` |

What each tool computes, and the forms it accepts, are in [docs/requirements.md](docs/requirements.md); the reasons behind them, including the tests with a small model that settled the tool definitions, are in [docs/why.md](docs/why.md).

## Status

The first version, 0.1.0. Written in Rust; one binary, no data kept.

## Run

jade-figuring runs in Docker, with `docker compose` or as a Portainer stack. The image is `ghcr.io/parkviewlab/jade-figuring`, for linux/amd64, published by each release; it may also be built from a checkout, with `docker build -t ghcr.io/parkviewlab/jade-figuring:latest .`.

```bash
docker compose up -d          # set JADE_ALLOWED_HOSTS in docker-compose.yml first
docker compose ps             # "healthy" within a few seconds
curl http://127.0.0.1:35833/health
```

An MCP client connects to `http://<host>:35833/mcp` (streamable HTTP), where `<host>` is one of the names in `JADE_ALLOWED_HOSTS`. For Portainer, paste [`docker-compose.yml`](docker-compose.yml) into the stack's web editor. The server keeps nothing between calls, so the stack has no volume. It has no login: run it on a private network, not on the public internet.

Without Docker, `cargo run --release` serves on `127.0.0.1:35833`.

## Endpoints

- `POST /mcp`: MCP over streamable HTTP, without sessions, with JSON responses; other methods answer 405. A request whose `Host` is not in `JADE_ALLOWED_HOSTS` is refused with 403, as is one carrying a browser `Origin` not in `JADE_ALLOWED_ORIGINS`, so that a web page cannot drive the server.
- `GET /health`: `{ok, version, uptime_seconds}`, which the image's health check (`jade-figuring health`) requests.
- `GET /admin/version`: `{name, version, tools, limits}`.

## Configuration

Every variable is read from the environment at start-up. The image sets `HOST` and `PORT`.

| Variable | Default | Purpose |
|---|---|---|
| `HOST` | `127.0.0.1` (image: `0.0.0.0`) | bind address |
| `PORT` | `35833` | listen port |
| `JADE_ALLOWED_HOSTS` | `localhost,127.0.0.1,::1` | comma-separated names (or `name:port`) clients use to reach the server; `*` allows any |
| `JADE_ALLOWED_ORIGINS` | empty: every browser `Origin` is refused | comma-separated origins allowed to call from a web page, such as `http://localhost:*` |

The calculators' behaviour has no settings: radians by default, 10 significant digits, whole numbers exact up to 300 digits, and the limits on expression length (1,000 characters), size (10,000 digits) and time (2 seconds) are constants, reported by `/admin/version`.

## Releasing

Tag-driven via the `Release` workflow on push of a `v*` tag. Use the [`ParkviewLab/dev-tools`](https://github.com/ParkviewLab/dev-tools) helpers; `VERSION.txt` is the only place the version lives (the crate's own stays `0.0.0`), and the workflow's gate refuses a tag that does not match it. A release runs from the `jade-figuring-main` worktree, in the handbook's flow:

```sh
git pull --ff-only                               # sync main
git -C ../jade-figuring-develop pull --ff-only   # sync develop
git merge --no-ff develop                        # promote develop to main
git bump <patch|minor|major|release>             # bumps VERSION.txt and commits "release vX.Y.Z"
git release                                      # annotated tag vX.Y.Z from VERSION.txt
git push --follow-tags                           # the tag push fires the workflow
```

A first release has no `git bump`. The release's last step is `git back-merge`, which opens the pull request that brings the release into `develop` and starts the next development cycle; see the handbook's [`releases.md`](https://github.com/ParkviewLab/handbook/blob/main/docs/releases.md).

The workflow runs a gate (the tag equals the version; the tagged commit is on `main`; the version is greater than the previous tag), then a `docker` job that builds and pushes the image for linux/amd64 with the `X.Y.Z`, `X.Y` and `latest` tags, then a `changelog` job that writes the new section of `CHANGELOG.md`, commits it to `main`, and creates the GitHub Release. Pull request titles carry the [Conventional Commit](https://www.conventionalcommits.org/) prefix the changelog reads; see [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md).

## License

jade-figuring is dual-licensed: the code under [AGPL-3.0-or-later](LICENSE) by default, with a commercial licence available as an alternative; the documentation under CC-BY-4.0. See [LICENSING.md](LICENSING.md).

---
<sub>© 2026 Gary Frattarola · part of [ParkviewLab](https://github.com/ParkviewLab)</sub>
