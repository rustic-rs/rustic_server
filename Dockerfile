FROM alpine AS builder
ARG RUSTIC_SERVER_VERSION
ARG TARGETARCH
RUN if [ "$TARGETARCH" = "amd64" ]; then \
        ASSET="rustic_server-x86_64-unknown-linux-musl.tar.xz";\
    elif [ "$TARGETARCH" = "arm64" ]; then \
        ASSET="rustic_server-aarch64-unknown-linux-musl.tar.xz"; \
    fi; \
    wget https://github.com/rustic-rs/rustic_server/releases/download/${RUSTIC_SERVER_VERSION}/${ASSET} && \
    tar -xf ${ASSET} --strip-components=1 && \
    mkdir /etc_files && \
    touch /etc_files/passwd && \
    touch /etc_files/group

FROM scratch
COPY --from=builder /usr/bin/wget /wget
COPY --from=builder /rustic-server /
COPY --from=builder /etc_files/ /etc/
EXPOSE 8080
ENTRYPOINT ["/rustic-server"]
HEALTHCHECK --interval=30s --timeout=10s --retries=3 \
  CMD /wget -q --spider http://localhost:8080/health/live
