# RFT - Robust File Transfer Protocol

RFT by Group 1

## Build

You need the rust toolchain (i.e. using [rustup.rs](https://rustup.rs)).

```
cargo build
```
To build the docs: `cargo docs`

## Usage
The command line application uses the command line options specified by the lecture slides.
The additional option `-l <list>` conflicts with `<file>...` and can be used to run a client for file list retrieval for the given directory instead of a client for regular file retrieval.
The additional option `-u` can be used by the server to specify an address that should be used for listening. 

### Print help:
`$ cargo run -- -h` / `$rft -h`

#### Output

```
rft 0.1.0
Peter Okelmann <okelmann@in.tum.de>
Joseph Birkner <joseph.birkner@tum.de>
Johannes Abel <abel@in.tum.de>
RFT - a rust implementation of the SOFT protocol

USAGE:
    rft [FLAGS] [OPTIONS] <host> <file>... -l <list> -s

FLAGS:
    -s               server mode: accept incoming files from any host
                     Operate in client mode if “–s” is not specified
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -t <t>           specify the port number to use (use a default if not given)
    -p <p>           'Packet n not Lost':
                     specify the loss probabilities for the Markov chain model
                     if only one is specified, assume p=q; if neither is specified assume no
                     loss
    -q <q>           'Packet n lost':
                     specify the loss probabilities for the Markov chain model
                     if only one is specified, assume p=q; if neither is specified assume no
                     loss
    -u <u>           address for listening
    -l <list>        remote directory for file list retrival

ARGS:
    <host>       the host to send to or request from (hostname or IPv4 address)
    <file>...    the name of the file(s) to fetch
```

### Run client for file retrieval:
`$ cargo run -- [OPTIONS] <host> <file>...`/ `$ rft [OPTIONS] <host> <file>...`

#### Example
`$ cargo run -- 127.0.0.1:42424 file1`

### Run server:
`$ cargo run -- [OPTIONS] -s`/ `$ rft [OPTIONS] -s`

#### Example
`$ cargo run -- -s -t 42424 -u 127.0.0.1`

### Run client for file list retrieval:
`$ cargo run -- [OPTIONS] <host> -l <list>`/ `$ rft [OPTIONS] <host> -l <list>`
