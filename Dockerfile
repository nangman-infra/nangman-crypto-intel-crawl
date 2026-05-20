# syntax=docker/dockerfile:1

FROM public.ecr.aws/docker/library/rust:1.94-bookworm AS builder

WORKDIR /opt/nangman-crypto/intel-crawl

COPY . /opt/nangman-crypto/intel-crawl

RUN cargo build --release

FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

COPY --from=builder \
    /etc/ssl/certs/ca-certificates.crt \
    /etc/ssl/certs/ca-certificates.crt

COPY --from=builder \
    /opt/nangman-crypto/intel-crawl/target/release/intel-crawl-app \
    /usr/local/bin/intel-crawl-app
COPY --from=builder \
    /opt/nangman-crypto/intel-crawl/config/source-registry.rss-seed.v1.json \
    /opt/nangman-crypto/intel-crawl/config/source-registry.rss-seed.v1.json

USER nonroot:nonroot

ENTRYPOINT ["/usr/local/bin/intel-crawl-app"]
