FROM rust:1.85-bookworm AS builder

WORKDIR /workspace
COPY . .
RUN cargo build --locked --release -p scotoma-relay

FROM debian:bookworm-slim AS runtime

RUN groupadd --system --gid 10001 scotoma \
    && useradd --system --uid 10001 --gid scotoma --no-create-home scotoma
COPY --from=builder /workspace/target/release/scotoma-relay /usr/local/bin/scotoma-relay

USER 10001:10001
EXPOSE 8787

ENTRYPOINT ["/usr/local/bin/scotoma-relay"]
