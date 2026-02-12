use eframe::egui;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use sysinfo::System;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "rproxy - Process Proxy Launcher",
        options,
        Box::new(|cc| {
            setup_chinese_font(&cc.egui_ctx);
            Ok(Box::new(ProxyLauncherApp::new()))
        }),
    )
}

fn setup_chinese_font(ctx: &egui::Context) {
    // 参考 egui-chinese-font: 使用可稳定显示中文的字体注入到 egui。
    // 当前环境无法在线拉取 crate 文档，因此这里保留本地字体加载作为兜底实现。
    let mut fonts = egui::FontDefinitions::default();

    for (name, path) in windows_cjk_font_candidates() {
        if let Ok(bytes) = fs::read(path) {
            fonts
                .font_data
                .insert(name.to_string(), egui::FontData::from_owned(bytes).into());
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, name.to_string());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, name.to_string());
        }
    }

    ctx.set_fonts(fonts);
}

fn windows_cjk_font_candidates() -> [(&'static str, &'static str); 8] {
    [
        ("simhei", "C:/Windows/Fonts/simhei.ttf"),
        ("simkai", "C:/Windows/Fonts/simkai.ttf"),
        ("simsun", "C:/Windows/Fonts/simsun.ttc"),
        ("msyh", "C:/Windows/Fonts/msyh.ttc"),
        ("msyhbd", "C:/Windows/Fonts/msyhbd.ttc"),
        ("msyh-ui", "C:/Windows/Fonts/msyh.ttc"),
        ("deng", "C:/Windows/Fonts/Deng.ttf"),
        ("dengb", "C:/Windows/Fonts/Dengb.ttf"),
    ]
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Serialize, Deserialize)]
struct ProxyProfile {
    name: String,
    ip: String,
    port: String,
    protocol: ProxyProtocol,
}

#[derive(Default, Serialize, Deserialize)]
struct AppConfig {
    profiles: Vec<ProxyProfile>,
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
    profiles: Vec<ProxyProfile>,
    selected_profile_index: Option<usize>,
    profile_name: String,
}

impl ProxyLauncherApp {
    fn new() -> Self {
        let config = load_config();
        let mut app = Self {
            ip: "127.0.0.1".to_string(),
            port: "7890".to_string(),
            protocol: ProxyProtocol::Http,
            processes: Vec::new(),
            selected_index: None,
            args: String::new(),
            status: "请选择进程并启动。".to_string(),
            profiles: config.profiles,
            selected_profile_index: None,
            profile_name: "默认配置".to_string(),
        };
        app.refresh_processes();
        app
    }

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

    fn save_new_profile(&mut self) {
        if self.profile_name.trim().is_empty() {
            self.status = "配置名称不能为空".to_string();
            return;
        }

        let profile = ProxyProfile {
            name: self.profile_name.trim().to_string(),
            ip: self.ip.trim().to_string(),
            port: self.port.trim().to_string(),
            protocol: self.protocol,
        };

        self.profiles.push(profile);
        self.selected_profile_index = Some(self.profiles.len() - 1);
        if let Err(err) = save_config(&AppConfig {
            profiles: self.profiles.clone(),
        }) {
            self.status = format!("保存配置失败: {err}");
            return;
        }

        self.status = "新增配置成功".to_string();
    }

    fn update_selected_profile(&mut self) {
        let idx = match self.selected_profile_index {
            Some(i) => i,
            None => {
                self.status = "请先在下拉框中选择一个配置".to_string();
                return;
            }
        };

        if let Some(profile) = self.profiles.get_mut(idx) {
            profile.name = self.profile_name.trim().to_string();
            profile.ip = self.ip.trim().to_string();
            profile.port = self.port.trim().to_string();
            profile.protocol = self.protocol;
        }

        if let Err(err) = save_config(&AppConfig {
            profiles: self.profiles.clone(),
        }) {
            self.status = format!("修改配置失败: {err}");
            return;
        }

        self.status = "修改配置成功".to_string();
    }

    fn delete_selected_profile(&mut self) {
        let idx = match self.selected_profile_index {
            Some(i) => i,
            None => {
                self.status = "请先选择要删除的配置".to_string();
                return;
            }
        };

        if idx < self.profiles.len() {
            self.profiles.remove(idx);
        }

        self.selected_profile_index = None;

        if let Err(err) = save_config(&AppConfig {
            profiles: self.profiles.clone(),
        }) {
            self.status = format!("删除配置失败: {err}");
            return;
        }

        self.status = "删除配置成功".to_string();
    }

    fn load_selected_profile_to_form(&mut self) {
        let idx = match self.selected_profile_index {
            Some(i) => i,
            None => {
                self.status = "请先选择要加载的配置".to_string();
                return;
            }
        };

        if let Some(profile) = self.profiles.get(idx) {
            self.profile_name = profile.name.clone();
            self.ip = profile.ip.clone();
            self.port = profile.port.clone();
            self.protocol = profile.protocol;
            self.status = "已加载配置到当前输入框".to_string();
        }
    }
}

impl eframe::App for ProxyLauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("rproxy - 进程代理启动器");
            ui.label("通过设置代理环境变量启动目标进程（HTTP/HTTPS/ALL_PROXY）。");
            ui.separator();

            ui.group(|ui| {
                ui.label("代理配置持久化");
                ui.horizontal(|ui| {
                    ui.label("配置名称:");
                    ui.text_edit_singleline(&mut self.profile_name);
                });

                egui::ComboBox::from_label("已保存配置")
                    .selected_text(
                        self.selected_profile_index
                            .and_then(|i| self.profiles.get(i))
                            .map(|p| p.name.clone())
                            .unwrap_or_else(|| "请选择".to_string()),
                    )
                    .show_ui(ui, |ui| {
                        for (idx, profile) in self.profiles.iter().enumerate() {
                            ui.selectable_value(
                                &mut self.selected_profile_index,
                                Some(idx),
                                profile.name.clone(),
                            );
                        }
                    });

                ui.horizontal(|ui| {
                    if ui.button("新增").clicked() {
                        self.save_new_profile();
                    }
                    if ui.button("修改").clicked() {
                        self.update_selected_profile();
                    }
                    if ui.button("删除").clicked() {
                        self.delete_selected_profile();
                    }
                    if ui.button("加载到输入框").clicked() {
                        self.load_selected_profile_to_form();
                    }
                });
            });

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

fn config_file_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("rproxy").join("profiles.json")
}

fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn load_config() -> AppConfig {
    let path = config_file_path();
    let Ok(content) = fs::read_to_string(path) else {
        return AppConfig::default();
    };

    serde_json::from_str::<AppConfig>(&content).unwrap_or_default()
}

fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_file_path();
    ensure_parent_dir(&path)?;
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::{split_args, AppConfig, ProxyProfile, ProxyProtocol};

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

    #[test]
    fn config_roundtrip() {
        let cfg = AppConfig {
            profiles: vec![ProxyProfile {
                name: "办公室代理".to_string(),
                ip: "10.10.10.1".to_string(),
                port: "8080".to_string(),
                protocol: ProxyProtocol::Http,
            }],
        };

        let json = serde_json::to_string(&cfg).expect("serialize config");
        let parsed: AppConfig = serde_json::from_str(&json).expect("deserialize config");
        assert_eq!(parsed.profiles.len(), 1);
        assert_eq!(parsed.profiles[0].name, "办公室代理");
    }
}
