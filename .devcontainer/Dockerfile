FROM rust:1.88.0

ARG USERNAME=dev
ARG USER_UID=1000
ARG USER_GID=$USER_UID
ARG DEBIAN_FRONTEND=noninteractive

ENV TZ="Europe/London"

RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME -s /bin/bash \
    && usermod -a -G dialout $USERNAME \
    #
    # [Optional] Add sudo support. Omit if you don't need to install software after connecting.
    && apt-get update \
    && apt-get install -y sudo \
    && echo $USERNAME ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/$USERNAME \
    && chmod 0440 /etc/sudoers.d/$USERNAME

RUN apt update && \
 apt install -y protobuf-compiler iputils-ping graphviz default-jre-headless vim perf-tools-unstable valgrind && \
 rustup component add clippy
RUN rustup toolchain install nightly && rustup component add rustfmt --toolchain nightly

# Disable ASLR
#COPY ./01-disable-aslr.conf /etc/sysctl.d/


USER ${USERNAME}
#RUN cargo install cargo-tarpaulin cargo-watch cargo-udeps cargo-feature cargo-valgrind cargo2junit cargo-chef flamegraph
RUN cargo install cargo-tarpaulin cargo-watch cargo-udeps cargo-feature cargo-valgrind cargo2junit cargo-chef

#RUN apk add --no-cache git
