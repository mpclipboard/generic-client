cbindgen:
    cbindgen --output mpclipboard-generic-client.h

example-c:
    cargo build
    cc examples/cli.c target/debug/libmpclipboard_generic_client.a -o cli-c
    RUST_LOG=trace ./cli-c

example-rs:
    RUST_LOG=trace cargo run --example cli --features libc

valgrind:
    cargo build --example cli --features libc
    valgrind --leak-check=full target/debug/examples/cli
