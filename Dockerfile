FROM rust:1.91-trixie

WORKDIR /rgo

RUN apt-get update && \
    apt-get install -y nasm binutils make && \
    rm -rf /var/lib/apt/lists/*

# Copy compiler source
COPY . .

# Build + install compiler binary (named `compiler`)
RUN cargo install --path .
ENV PATH="/usr/local/cargo/bin:${PATH}"

# Install wrapper
COPY compile.sh /usr/local/bin/rgo-compile
RUN chmod +x /usr/local/bin/rgo-compile

ENTRYPOINT ["/usr/local/bin/rgo-compile"]
