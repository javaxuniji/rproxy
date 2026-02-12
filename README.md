# rproxy

A Rust desktop tool built with `eframe/egui` for launching a process with proxy environment variables.

## Features

- Configure proxy IP, port, and protocol (HTTP / SOCKS5 / SOCKS4).
- List currently running processes and select a target process.
- Relaunch the selected executable with the following variables set:
  - `HTTP_PROXY`
  - `HTTPS_PROXY`
  - `ALL_PROXY`
- Optional startup arguments.

> Note: this approach relies on inherited environment variables, so it only affects newly launched processes.

## Run

```bash
cargo run
```

## Build

```bash
cargo build --release
```
