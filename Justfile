cbindgen:
    cbindgen --output mpclipboard-generic-client.h

example-c:
    cargo build
    clang examples/cli.c target/debug/libmpclipboard_generic_client.a -I . -framework Security -framework CoreFoundation -o cli-c
    RUST_LOG=trace ./cli-c
