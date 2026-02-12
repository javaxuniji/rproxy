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
            status: "Select a process and launch it.".to_string(),
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
            return Err("IP address cannot be empty".to_string());
        }

        let port = self
            .port
            .trim()
            .parse::<u16>()
            .map_err(|_| "Invalid port (1-65535)".to_string())?;

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
                self.status = "Please select a process first".to_string();
                return;
            }
        };

        let exe_path = match selected.executable_path() {
            Some(path) if path.exists() => path,
            _ => {
                self.status =
                    "The selected process has no executable path and cannot be relaunched"
                        .to_string();
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
                    "Started [{}] pid={} with proxy={}. Note: only newly launched process inherits these variables.",
                    selected.name,
                    child.id(),
                    proxy
                );
            }
            Err(err) => {
                self.status = format!("Launch failed: {err}");
            }
        }
    }
}

impl eframe::App for ProxyLauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("rproxy - Process Proxy Launcher");
            ui.label(
                "Launch a process with HTTP_PROXY / HTTPS_PROXY / ALL_PROXY environment variables.",
            );
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Proxy IP:");
                ui.text_edit_singleline(&mut self.ip);
                ui.label("Port:");
                ui.text_edit_singleline(&mut self.port);
            });

            egui::ComboBox::from_label("Proxy protocol")
                .selected_text(self.protocol.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.protocol, ProxyProtocol::Http, "HTTP");
                    ui.selectable_value(&mut self.protocol, ProxyProtocol::Socks5, "SOCKS5");
                    ui.selectable_value(&mut self.protocol, ProxyProtocol::Socks4, "SOCKS4");
                });

            ui.horizontal(|ui| {
                if ui.button("Refresh process list").clicked() {
                    self.refresh_processes();
                }
                if let Ok(proxy) = self.current_proxy_url() {
                    ui.label(format!("Current proxy: {proxy}"));
                }
            });

            egui::ScrollArea::vertical()
                .max_height(220.0)
                .show(ui, |ui| {
                    for (idx, process) in self.processes.iter().enumerate() {
                        let selected = self.selected_index == Some(idx);
                        if ui
                            .selectable_label(selected, process.display_text())
                            .on_hover_text("Relaunch using this executable")
                            .clicked()
                        {
                            self.selected_index = Some(idx);
                        }
                    }
                });

            ui.separator();
            ui.label("Optional args (appended when launching):");
            ui.text_edit_singleline(&mut self.args);

            if ui.button("Launch selected process with proxy").clicked() {
                self.launch_with_proxy();
            }

            ui.separator();
            ui.label(format!("Status: {}", self.status));
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
