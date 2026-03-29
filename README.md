# rusty_s3

A minimal, dependency-free S3-compatible server written in Rust. This is a learning exercise — not intended for production use.

## Goals

- Implement core S3 operations from scratch using only Rust's standard library
- Learn low-level HTTP parsing, TCP networking, and the S3 protocol
- Zero external dependencies

## Current State

This project is in its very early stages. So far:

- **TCP server** listening on `127.0.0.1:9100`
- **Hand-rolled HTTP request parser** (method, path, headers, body size)
- Case-insensitive header lookup
- No S3 operations implemented yet — just the networking foundation

## Building & Running

```sh
cargo build
cargo run
```

The server starts on `http://127.0.0.1:9100`.

## License

This project is unlicensed — it's a personal learning exercise.
