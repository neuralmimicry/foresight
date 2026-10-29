# Prebuild with the locked toolchain for the target architecture. No shell,
# package manager or emulation is needed in the production runtime.
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:3ee00291cf386ec52a552e6a858d555debd74776eb4bfa53fa3df2cd6677515e
ARG SOURCE_REVISION
LABEL org.opencontainers.image.source="https://github.com/neuralmimicry/foresight"
LABEL org.opencontainers.image.revision="${SOURCE_REVISION}"
COPY --chmod=0555 dist/foresight /usr/local/bin/foresight
USER 65532:65532
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/foresight"]

