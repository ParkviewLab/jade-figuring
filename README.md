<!--
SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
SPDX-License-Identifier: CC-BY-4.0
-->

# jade-figuring

A calculator MCP server for language models: an engineering calculator and date arithmetic, offered as two tools with definitions small enough for a model with a short context. A model passes an ordinary expression, written as it would write one (`2^100`, `sin(30 degrees)`, `52 choose 5`, LaTeX, `days between 2026-10-31 and 2026-12-25`), and receives an exact result to 10 significant digits, with the expression as it was read.

## Status

In development; the first release is 0.1.0. The requirements are in [docs/requirements.md](docs/requirements.md) and the reasons behind them in [docs/why.md](docs/why.md).

## License

jade-figuring is dual-licensed: the code under [AGPL-3.0-or-later](LICENSE) by default, with a commercial licence available as an alternative; the documentation under CC-BY-4.0. See [LICENSING.md](LICENSING.md).

---
<sub>© 2026 Gary Frattarola · part of [ParkviewLab](https://github.com/ParkviewLab)</sub>
