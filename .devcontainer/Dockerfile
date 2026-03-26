FROM rust:1-slim-bookworm

ARG USER=devcont

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        git \
        curl \
        build-essential \
        pkg-config \
        ca-certificates \
        sudo \
        docker.io \
        podman \
        podman-compose && \
    rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-llvm-cov cargo-deny

RUN adduser --disabled-password --gecos "" $USER && \
    echo "${USER} ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers && \
    usermod -aG docker $USER && \
    echo "devcont:100000:65536" >> /etc/subuid && \
    echo "devcont:100000:65536" >> /etc/subgid

WORKDIR /workspace

USER $USER
