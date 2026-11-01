// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Embeds the version from VERSION.txt, the repository's one source of the version.

fn main() {
    println!("cargo:rerun-if-changed=VERSION.txt");
    let version = std::fs::read_to_string("VERSION.txt").expect("VERSION.txt is readable");
    println!("cargo:rustc-env=JADE_FIGURING_VERSION={}", version.trim());
}
