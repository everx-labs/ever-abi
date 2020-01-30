FROM alpine:latest
RUN addgroup --gid 1000 jenkins && \
    adduser -D -G jenkins jenkins
COPY --chown=jenkins:jenkins ./Cargo.* ./*.md ./*.rs /ton-labs-abi/
COPY --chown=jenkins:jenkins ./src /ton-labs-abi/src
VOLUME ["/ton-labs-abi"]
USER jenkins