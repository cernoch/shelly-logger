# Build image
FROM messense/rust-musl-cross:x86_64-musl AS builder
WORKDIR /usr/src/
RUN rustup target add x86_64-unknown-linux-musl

# Need openssl linked against musl, not glibc.
# https://github.com/rust-cross/rust-musl-cross/commit/f948984ca3d4c5251dfbcd0781014f7e85f14c2b#diff-dd2c0eb6ea5cfc6c4bd4eac30934e2d5746747af48fef6da689e85b752f39557
ENV OPENSSL_ARCH=linux-x86_64
RUN export CC=$TARGET_CC && \
    export C_INCLUDE_PATH=$TARGET_C_INCLUDE_PATH && \
    export LD=$TARGET-ld && \
    echo "Building OpenSSL" && \
    OPENSSL_VERSION=1.0.2u && \
    CHECKSUM=ecd0c6ffb493dd06707d38b14bb4d8c2288bb7033735606569d8f90f89669d16 && \
    curl -sqO https://www.openssl.org/source/openssl-$OPENSSL_VERSION.tar.gz && \
    echo "$CHECKSUM openssl-$OPENSSL_VERSION.tar.gz" > checksums.txt && \
    sha256sum -c checksums.txt && \
    tar xzf openssl-$OPENSSL_VERSION.tar.gz && cd openssl-$OPENSSL_VERSION && \
    ./Configure $OPENSSL_ARCH -fPIC --prefix=$TARGET_HOME && \
    make -j$(nproc) && make install && \
    cd .. && rm -rf openssl-$OPENSSL_VERSION.tar.gz openssl-$OPENSSL_VERSION checksums.txt
ENV OPENSSL_DIR=$TARGET_HOME/ \
    OPENSSL_INCLUDE_DIR=$TARGET_HOME/include/ \
    DEP_OPENSSL_INCLUDE=$TARGET_HOME/include/ \
    OPENSSL_LIB_DIR=$TARGET_HOME/lib/ \
    OPENSSL_STATIC=1

# Build rust dependencies and cache them
RUN USER=root cargo new shelly-logger
WORKDIR /usr/src/shelly-logger
COPY ./app/Cargo.toml ./app/Cargo.lock ./
RUN cargo build --target x86_64-unknown-linux-musl --release && rm -rf src
# Clean cache and build the application only
COPY ./app/src ./src
RUN rm -rf target/x86_64-unknown-linux-musl/release/.fingerprint/shelly-logger-*\
 && cargo build --target x86_64-unknown-linux-musl --release

# Run image
FROM scratch
COPY ./app/config.json /etc/shelly-logger/
COPY --from=builder /usr/src/shelly-logger/target/x86_64-unknown-linux-musl/release/shelly-logger /usr/local/bin/
ENV RUST_LOG=info
WORKDIR /etc/shelly-logger
CMD ["/usr/local/bin/shelly-logger"]
