# rproxy

使用 Rust + `eframe/egui` 实现的桌面代理启动器（Windows 11 友好）。

## 功能

- 输入代理服务器 IP、端口、协议（HTTP / SOCKS5 / SOCKS4）。
- 列出系统当前进程并选择目标进程。
- 以代理环境变量重新启动目标可执行文件：
  - `HTTP_PROXY`
  - `HTTPS_PROXY`
  - `ALL_PROXY`
- 支持配置持久化（保存到本地 `profiles.json`），并提供：
  - 新增配置
  - 修改配置
  - 删除配置
  - 加载配置到输入框
- Windows 常见中文字体自动加载（微软雅黑/宋体/等线等候选），避免中文显示乱码。

> 注意：该工具通过“环境变量继承”实现代理，仅对新启动进程生效，无法强制接管已经运行进程的网络流量。

## 运行

```bash
cargo run
```

## 构建

```bash
cargo build --release
```
