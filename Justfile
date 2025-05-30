cbindgen:
    cbindgen --output shared-clipboard-client-generic.h

shared-client-example-c-macos:
    cargo build
    clang examples/shared-client-example-c.c target/debug/libshared_clipboard_client_generic.a -I . -framework Security -framework CoreFoundation -o target/debug/shared-client-example-c
    RUST_LOG=trace ./target/debug/shared-client-example-c
