FROM rust:1.91-trixie

WORKDIR /rgo

# Install minimal toolchain
RUN apt-get update && \
    apt-get install -y nasm binutils make && \
    rm -rf /var/lib/apt/lists/*

# Copy compiler source from the user's cloned repo
COPY . .

# Build the compiler in release mode
RUN cargo build --release

# Install the compile wrapper
COPY compile.sh /usr/local/bin/rgo-compile
RUN chmod +x /usr/local/bin/rgo-compile

ENTRYPOINT ["rgo-compile"]
