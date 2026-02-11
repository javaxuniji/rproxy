# rproxy

使用 Rust + `eframe/egui` 实现的桌面代理启动器。

## 功能

- 输入代理服务器的 IP、端口、协议（HTTP / SOCKS5 / SOCKS4）。
- 列出系统当前进程，选择一个目标进程。
- 读取目标进程的可执行文件路径，并以代理环境变量重新启动该可执行文件：
  - `HTTP_PROXY`
  - `HTTPS_PROXY`
  - `ALL_PROXY`
- 可附加启动参数。

> 注意：该工具通过“环境变量继承”实现代理，**仅对新启动的进程生效**，无法强制接管已经运行的进程网络流量。

## 运行

```bash
cargo run
```

## 构建

```bash
cargo build --release
```
