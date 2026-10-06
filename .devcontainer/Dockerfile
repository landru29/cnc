FROM rust:1.98.1-trixie

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    zsh \
    sudo \
    socat \
    git

RUN groupadd --gid 1000 vscode \
    && useradd --uid 1000 --gid 1000 --create-home --shell /bin/bash vscode \
    && mkdir -p /workspace /go \
    && chown -R vscode:vscode /workspace /go

USER vscode
ENV PATH="/go/bin:${PATH}"
WORKDIR /workspace

RUN sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" "" --unattended --skip-chsh

