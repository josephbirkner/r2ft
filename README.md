# RFT - Robust File Transfer Protocol

RFT by Group 1

## Build

You need the rust toolchain (i.e. using [rustup.rs](https://rustup.rs)).

```
cargo build
```

To build the docs: `cargo docs`

## Usage

Client: `cargo run -- 127.0.0.1:42424 test.txt`

Server: `cargo run -- -s`

