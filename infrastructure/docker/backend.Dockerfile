FROM rust:1.100-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --locked --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/inter-pj-manager /usr/local/bin/manager
WORKDIR /app
COPY --from=build /src/migrations ./migrations
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/manager"]
