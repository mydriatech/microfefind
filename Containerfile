FROM docker.io/library/rust:alpine as builder
WORKDIR /work
COPY . .
RUN \
    apk add musl-dev curl && \
    cargo update && \
    cargo build --target=x86_64-unknown-linux-musl --release && \
    ./bin/extract-third-party-licenses.sh

FROM scratch

LABEL org.opencontainers.image.source="https://github.com/mydriatech/microfefind"
LABEL org.opencontainers.image.description="Micro front end discovery on Kubernetes"
LABEL org.opencontainers.image.licenses="Apache-2.0 WITH FWM-Exception-1.0.0 AND Apache-2.0 AND Apache-2.0 WITH LLVM-exception AND BSD-3-Clause AND ISC AND MIT AND Unicode-DFS-2016 AND OpenSSL"
LABEL org.opencontainers.image.vendor="MydriaTech AB"

COPY --from=builder --chown=10001:0 /work/target/x86_64-unknown-linux-musl/release/microfefind /microfefind
COPY --from=builder --chown=10001:0 --chmod=770 /work/licenses /licenses

WORKDIR /

USER 10001:0

EXPOSE 8083

#ENV APP_NAME                             "microfefind"

ENV MICROFEFIND_LOG_LEVEL                "INFO"

ENV MICROFEFIND_API_PORT                 "8083"
ENV MICROFEFIND_API_ADDRESS              "0.0.0.0"

ENV MICROFEFIND_INGRESS_LABELS           "microfe=true"
ENV MICROFEFIND_INGRESS_ANNOTATIONPREFIX "microfe/"
ENV MICROFEFIND_INGRESS_NAMESPACES       ""

CMD ["/microfefind"]
