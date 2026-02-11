use eframe::egui;
use std::path::PathBuf;
use std::process::Command;
use sysinfo::System;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "rproxy - Process Proxy Launcher",
        options,
        Box::new(|_cc| Ok(Box::<ProxyLauncherApp>::default())),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProxyProtocol {
    Http,
    Socks5,
    Socks4,
}

impl ProxyProtocol {
    fn as_scheme(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Socks5 => "socks5",
            Self::Socks4 => "socks4",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Http => "HTTP",
            Self::Socks5 => "SOCKS5",
            Self::Socks4 => "SOCKS4",
        }
    }
}

#[derive(Clone)]
struct ProcessInfo {
    pid: String,
    name: String,
    exe: Option<PathBuf>,
}

impl ProcessInfo {
    fn display_text(&self) -> String {
        match &self.exe {
            Some(exe) => format!("{} ({}) - {}", self.name, self.pid, exe.display()),
            None => format!("{} ({})", self.name, self.pid),
        }
    }

    fn executable_path(&self) -> Option<PathBuf> {
        self.exe.clone()
    }
}

struct ProxyLauncherApp {
    ip: String,
    port: String,
    protocol: ProxyProtocol,
    processes: Vec<ProcessInfo>,
    selected_index: Option<usize>,
    args: String,
    status: String,
}

impl Default for ProxyLauncherApp {
    fn default() -> Self {
        let mut app = Self {
            ip: "127.0.0.1".to_string(),
            port: "7890".to_string(),
            protocol: ProxyProtocol::Http,
            processes: Vec::new(),
            selected_index: None,
            args: String::new(),
            status: "请选择进程并启动。".to_string(),
        };
        app.refresh_processes();
        app
    }
}

impl ProxyLauncherApp {
    fn refresh_processes(&mut self) {
        let mut system = System::new_all();
        system.refresh_all();

        let mut processes = system
            .processes()
            .iter()
            .map(|(pid, process)| ProcessInfo {
                pid: pid.to_string(),
                name: process.name().to_string_lossy().to_string(),
                exe: process.exe().map(|p| p.to_path_buf()),
            })
            .collect::<Vec<_>>();

        processes.sort_by(|a, b| a.name.cmp(&b.name));
        self.processes = processes;

        if let Some(idx) = self.selected_index {
            if idx >= self.processes.len() {
                self.selected_index = None;
            }
        }
    }

    fn current_proxy_url(&self) -> Result<String, String> {
        if self.ip.trim().is_empty() {
            return Err("IP 地址不能为空".to_string());
        }

        let port = self
            .port
            .trim()
            .parse::<u16>()
            .map_err(|_| "端口号无效（1-65535）".to_string())?;

        Ok(format!(
            "{}://{}:{}",
            self.protocol.as_scheme(),
            self.ip.trim(),
            port
        ))
    }

    fn launch_with_proxy(&mut self) {
        let proxy = match self.current_proxy_url() {
            Ok(proxy) => proxy,
            Err(err) => {
                self.status = err;
                return;
            }
        };

        let selected = match self
            .selected_index
            .and_then(|idx| self.processes.get(idx))
            .cloned()
        {
            Some(info) => info,
            None => {
                self.status = "请先选择一个进程".to_string();
                return;
            }
        };

        let exe_path = match selected.executable_path() {
            Some(path) if path.exists() => path,
            _ => {
                self.status = "所选进程没有可执行文件路径，无法重启为代理模式".to_string();
                return;
            }
        };

        let args = split_args(self.args.trim());
        let mut command = Command::new(&exe_path);
        command.args(args);
        command
            .env("HTTP_PROXY", &proxy)
            .env("HTTPS_PROXY", &proxy)
            .env("ALL_PROXY", &proxy)
            .env("http_proxy", &proxy)
            .env("https_proxy", &proxy)
            .env("all_proxy", &proxy)
            .env("NO_PROXY", "")
            .env("no_proxy", "");

        if let Some(parent) = exe_path.parent() {
            command.current_dir(parent);
        }

        match command.spawn() {
            Ok(child) => {
                self.status = format!(
                    "已启动 [{}] pid={}，代理={}。注意：仅新启动进程会继承代理环境变量。",
                    selected.name,
                    child.id(),
                    proxy
                );
            }
            Err(err) => {
                self.status = format!("启动失败: {err}");
            }
        }
    }
}

impl eframe::App for ProxyLauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("rproxy - 进程代理启动器");
            ui.label("通过设置代理环境变量启动目标进程（HTTP/HTTPS/ALL_PROXY）。");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("代理 IP:");
                ui.text_edit_singleline(&mut self.ip);
                ui.label("端口:");
                ui.text_edit_singleline(&mut self.port);
            });

            egui::ComboBox::from_label("代理协议")
                .selected_text(self.protocol.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.protocol, ProxyProtocol::Http, "HTTP");
                    ui.selectable_value(&mut self.protocol, ProxyProtocol::Socks5, "SOCKS5");
                    ui.selectable_value(&mut self.protocol, ProxyProtocol::Socks4, "SOCKS4");
                });

            ui.horizontal(|ui| {
                if ui.button("刷新进程列表").clicked() {
                    self.refresh_processes();
                }
                if let Ok(proxy) = self.current_proxy_url() {
                    ui.label(format!("当前代理: {proxy}"));
                }
            });

            egui::ScrollArea::vertical()
                .max_height(220.0)
                .show(ui, |ui| {
                    for (idx, process) in self.processes.iter().enumerate() {
                        let selected = self.selected_index == Some(idx);
                        if ui
                            .selectable_label(selected, process.display_text())
                            .on_hover_text("选择后将以该可执行文件重新启动")
                            .clicked()
                        {
                            self.selected_index = Some(idx);
                        }
                    }
                });

            ui.separator();
            ui.label("可选参数（启动时附加到可执行文件后）:");
            ui.text_edit_singleline(&mut self.args);

            if ui.button("使用代理启动选中进程").clicked() {
                self.launch_with_proxy();
            }

            ui.separator();
            ui.label(format!("状态: {}", self.status));
        });
    }
}

fn split_args(input: &str) -> Vec<String> {
    input
        .split_whitespace()
        .filter(|arg| !arg.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{split_args, ProxyProtocol};

    #[test]
    fn parse_args() {
        let args = split_args("--config config.toml  --debug");
        assert_eq!(args, vec!["--config", "config.toml", "--debug"]);
    }

    #[test]
    fn proxy_scheme() {
        assert_eq!(ProxyProtocol::Http.as_scheme(), "http");
        assert_eq!(ProxyProtocol::Socks5.as_scheme(), "socks5");
        assert_eq!(ProxyProtocol::Socks4.as_scheme(), "socks4");
    }
}
