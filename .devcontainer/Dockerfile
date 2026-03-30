FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    bash \
    git \
    curl \
    ca-certificates \
    gcc \
    libc6-dev \
    binutils \
    file \
    xxd \
    less \
    # GTK dependencies for Rust ATK bindings
    build-essential \
    pkg-config \
    libgtk-3-dev \
    libappindicator3-dev \
    libxdo-dev \
    libssl-dev \
    libwayland-dev \
    && apt-get -y autoremove \
    && rm -rf /var/lib/apt/lists/*

ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=$USER_UID
RUN groupadd --gid $USER_GID $USERNAME && \
    useradd --uid $USER_UID --gid $USER_GID -m -s /bin/bash $USERNAME

USER $USERNAME

ENV RUSTUP_HOME=/home/${USERNAME}/.rustup \
    CARGO_HOME=/home/${USERNAME}/.cargo \
    PATH=/home/${USERNAME}/.cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile default \
    && rustup component add rust-src rustfmt

WORKDIR /workspace
