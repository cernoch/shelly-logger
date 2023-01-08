FROM rust:1.66

COPY ./app /build
RUN mkdir /config \
 && cd build \
 && cargo build -r \
 && cargo install --path . \
 && cp config.json /config/ \
 && cd / && rm -rf build

ENV RUST_LOG=info
CMD cd /config && shelly-logger
