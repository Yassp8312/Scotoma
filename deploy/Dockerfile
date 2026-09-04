FROM rust:1.96.0-bookworm@sha256:5e2214abe154fe26e39f64488952e5c991eeed1d6d6da7cc8381ae83927f0cfc AS builder

WORKDIR /workspace
COPY . .
RUN cargo build --locked --release -p scotoma-relay

FROM debian:bookworm-slim@sha256:88200866dfff7ea7f5cbcb6ec7c8a701889efe6fe859fe64d6990e4b07ea4171 AS runtime

RUN groupadd --system --gid 10001 scotoma \
    && useradd --system --uid 10001 --gid scotoma --no-create-home scotoma
COPY --from=builder /workspace/target/release/scotoma-relay /usr/local/bin/scotoma-relay

USER 10001:10001
EXPOSE 8787

ENTRYPOINT ["/usr/local/bin/scotoma-relay"]
