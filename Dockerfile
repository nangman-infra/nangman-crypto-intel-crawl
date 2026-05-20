# syntax=docker/dockerfile:1

FROM public.ecr.aws/docker/library/rust:1.94-bookworm AS builder

WORKDIR /opt/nangman-crypto/intel-crawl

COPY . /opt/nangman-crypto/intel-crawl

RUN cargo build --release

FROM public.ecr.aws/docker/library/debian:bookworm-slim AS runtime

RUN mkdir -p /etc/ssl/certs \
    && mkdir -p /opt/nangman-crypto/intel-crawl/config \
    && chown -R 10001:10001 /opt/nangman-crypto

COPY --from=builder \
    /etc/ssl/certs/ca-certificates.crt \
    /etc/ssl/certs/ca-certificates.crt

COPY --from=builder \
    /opt/nangman-crypto/intel-crawl/target/release/intel-crawl-app \
    /usr/local/bin/intel-crawl-app
COPY --from=builder \
    /opt/nangman-crypto/intel-crawl/config/source-registry.rss-seed.v1.json \
    /opt/nangman-crypto/intel-crawl/config/source-registry.rss-seed.v1.json

USER 10001:10001

ENTRYPOINT ["/usr/local/bin/intel-crawl-app"]
