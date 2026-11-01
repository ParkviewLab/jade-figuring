<!--
SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
SPDX-License-Identifier: CC-BY-4.0
-->

# Licensing

jade-figuring is dual-licensed, and different parts of the repository carry different licences. This file is the human-readable guide; the exact, machine-readable licence texts live in [`LICENSES/`](LICENSES/), one file per licence, as the [REUSE](https://reuse.software) specification requires.

## The open-source option (default)

The code is free software under the GNU Affero General Public License, version 3 or (at your option) any later version (`AGPL-3.0-or-later`). You may use, study, modify and redistribute it under those terms.

Note the AGPL's network clause (section 13): if you run a modified version and let users interact with it over a network, you must offer them the corresponding source.

Full text: [`LICENSE`](LICENSE) (also `LICENSES/AGPL-3.0-or-later.txt`).

## The commercial option

If you cannot or prefer not to comply with the AGPL, for example to embed jade-figuring in a closed-source product or to avoid the source-disclosure obligation, a separate commercial licence is available from the copyright holder.

> Commercial licensing: Gary Frattarola, garyf@parkviewlab.ai

## What each part of the repository is licensed under

| Part | Licence |
|---|---|
| Source code, build script, Dockerfile, compose file, configuration, lockfile, workflows | `AGPL-3.0-or-later`, or the commercial licence above |
| Documentation (`docs/`, `README.md`, this file) | `CC-BY-4.0` |
| ParkviewLab logo and brand assets, should any be added | All rights reserved (`LicenseRef-AllRightsReserved`), not covered by the AGPL or the commercial grant |

Licensing is machine-verifiable: every file carries an SPDX tag or is annotated in [`REUSE.toml`](REUSE.toml), and the repository passes `reuse lint`.

### A note on the layout

- [`LICENSE`](LICENSE): a copy of the AGPL text at the repository root, kept so that GitHub detects and displays the licence.
- [`LICENSES/`](LICENSES/): the canonical licence texts for tooling (REUSE), one file per SPDX identifier.
- `LICENSING.md` (this file): the human explanation of the dual-licence model.

Copyright © 2026 Gary Frattarola <garyf@parkviewlab.ai>
