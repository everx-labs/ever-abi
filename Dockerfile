ARG TON_LABS_TYPES_IMAGE=tonlabs/ton-labs-types:latest
ARG TON_LABS_BLOCK_IMAGE=tonlabs/ton-labs-block:latest
ARG TON_LABS_ABI_IMAGE=tonlabs/ton-labs-abi:latest

FROM alpine:latest as ton-labs-abi-src
RUN addgroup --gid 1000 jenkins && \
    adduser -D -G jenkins jenkins
COPY --chown=jenkins:jenkins ./Cargo.* ./*.md ./*.rs /tonlabs/ton-labs-abi/
COPY --chown=jenkins:jenkins ./src /tonlabs/ton-labs-abi/src
VOLUME ["/tonlabs/ton-labs-abi"]
USER jenkins

FROM $TON_LABS_TYPES_IMAGE as ton-labs-types-src
FROM $TON_LABS_BLOCK_IMAGE as ton-labs-block-src
FROM $TON_LABS_ABI_IMAGE as ton-labs-abi-source

FROM alpine:latest as ton-labs-abi-full
RUN addgroup --gid 1000 jenkins && \
    adduser -D -G jenkins jenkins
COPY --from=ton-labs-types-src  --chown=jenkins:jenkins /tonlabs/ton-labs-types /tonlabs/ton-labs-types
COPY --from=ton-labs-block-src  --chown=jenkins:jenkins /tonlabs/ton-labs-block /tonlabs/ton-labs-block
COPY --from=ton-labs-abi-source --chown=jenkins:jenkins /tonlabs/ton-labs-abi   /tonlabs/ton-labs-abi
VOLUME ["/tonlabs"]

FROM rust:latest as ton-labs-abi-rust
RUN apt -qqy update && apt -qyy install apt-utils && \
    curl -sL https://deb.nodesource.com/setup_12.x | bash - && \
    apt-get install -qqy nodejs && \
    adduser --group jenkins && \
    adduser -q --disabled-password --gid 1000 jenkins && \
    mkdir /tonlabs && chown -R jenkins:jenkins /tonlabs
COPY --from=ton-labs-abi-full --chown=jenkins:jenkins /tonlabs/ton-labs-types /tonlabs/ton-labs-types
COPY --from=ton-labs-abi-full --chown=jenkins:jenkins /tonlabs/ton-labs-block /tonlabs/ton-labs-block
COPY --from=ton-labs-abi-full --chown=jenkins:jenkins /tonlabs/ton-labs-abi   /tonlabs/ton-labs-abi
WORKDIR /tonlabs/ton-labs-abi