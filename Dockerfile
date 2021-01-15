FROM rust
RUN rustup component add clippy
RUN rustup component add rustfmt