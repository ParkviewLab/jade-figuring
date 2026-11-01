# SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
# SPDX-License-Identifier: AGPL-3.0-or-later

# Build stage: the release binary.
FROM rust:1.99-slim-trixie AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock build.rs VERSION.txt ./
COPY src ./src
RUN cargo build --release --locked

# Runtime stage: the binary alone, as a non-root user. The server keeps no data.
FROM debian:trixie-slim
RUN useradd --system --uid 10001 --no-create-home jade
COPY --from=build /src/target/release/jade-figuring /usr/local/bin/jade-figuring
USER jade
ENV HOST=0.0.0.0 \
    PORT=35833
EXPOSE 35833
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD ["jade-figuring", "health"]
CMD ["jade-figuring"]
