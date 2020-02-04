FROM alpine:latest
RUN addgroup --gid 1000 jenkins && \
    adduser -D -G jenkins jenkins
COPY --chown=jenkins:jenkins ./Cargo.* ./*.md ./*.rs /tonlabs/ton-labs-abi/
COPY --chown=jenkins:jenkins ./src /tonlabs/ton-labs-abi/src
VOLUME ["/tonlabs/ton-labs-abi"]
USER jenkins