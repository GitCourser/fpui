use crate::config::{Config, Param};
use eframe::egui;
use std::{
    fs,
    process::Command,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct FpUiApp {
    config: Config,
    show_advanced: bool,
    status_message: String,
    status_color: egui::Color32,
}

impl FpUiApp {
    const WINDOW_WIDTH: f32 = 615.0;
    const COLLAPSED_HEIGHT: f32 = 310.0;
    const EXPANDED_HEIGHT: f32 = 545.0;
    const CHECKBOX_WIDTH: f32 = 24.0;
    const LABEL_WIDTH: f32 = 205.0;
    const INPUT_WIDTH: f32 = 340.0;
    const ROW_HEIGHT: f32 = 24.0;
    const RANDOM_BUTTON_WIDTH: f32 = 58.0;
    const LAUNCH_BUTTON_WIDTH: f32 = 92.0;

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::setup_chinese_fonts(cc);

        Self {
            config: Config::load(),
            show_advanced: false,
            status_message: String::new(),
            status_color: egui::Color32::GREEN,
        }
    }

    fn setup_chinese_fonts(cc: &eframe::CreationContext<'_>) {
        let font_candidates = [
            r"C:\\Windows\\Fonts\\msyh.ttc",
            r"C:\\Windows\\Fonts\\msyh.ttf",
            r"C:\\Windows\\Fonts\\simhei.ttf",
            r"C:\\Windows\\Fonts\\simsun.ttc",
        ];

        let Some(font_bytes) = font_candidates.iter().find_map(|path| fs::read(path).ok()) else {
            return;
        };

        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "windows_chinese".to_string(),
            Arc::new(egui::FontData::from_owned(font_bytes)),
        );

        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, "windows_chinese".to_string());
        }
        cc.egui_ctx.set_fonts(fonts);
    }

    fn set_window_height(ctx: &egui::Context, show_advanced: bool) {
        let height = if show_advanced {
            Self::EXPANDED_HEIGHT
        } else {
            Self::COLLAPSED_HEIGHT
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
            Self::WINDOW_WIDTH,
            height,
        )));
    }

    fn right_aligned_label(ui: &mut egui::Ui, label: &str) {
        ui.allocate_ui_with_layout(
            egui::vec2(Self::LABEL_WIDTH, Self::ROW_HEIGHT),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.label(label);
            },
        );
    }

    fn param_ui(ui: &mut egui::Ui, label: &str, param: &mut Param, hint: &str) {
        ui.horizontal(|ui| {
            ui.add_sized(
                [Self::CHECKBOX_WIDTH, Self::ROW_HEIGHT],
                egui::Checkbox::new(&mut param.enabled, ""),
            )
            .on_hover_text("启用/禁用此参数");
            Self::right_aligned_label(ui, label);
            ui.add_sized(
                [Self::INPUT_WIDTH, 20.0],
                egui::TextEdit::singleline(&mut param.value).hint_text(hint),
            );
        });
    }

    fn required_param_ui(ui: &mut egui::Ui, label: &str, param: &mut Param, hint: &str) {
        ui.horizontal(|ui| {
            ui.add_space(Self::CHECKBOX_WIDTH + ui.spacing().item_spacing.x);
            Self::right_aligned_label(ui, label);
            ui.add_sized(
                [Self::INPUT_WIDTH, 20.0],
                egui::TextEdit::singleline(&mut param.value).hint_text(hint),
            );
        });
    }

    fn select_param_ui(ui: &mut egui::Ui, label: &str, param: &mut Param, options: &[&str]) {
        ui.horizontal(|ui| {
            ui.add_space(Self::CHECKBOX_WIDTH + ui.spacing().item_spacing.x);
            Self::right_aligned_label(ui, label);
            let selected_text = if param.value.is_empty() {
                "无"
            } else {
                &param.value
            };
            egui::ComboBox::from_id_salt(label)
                .width(Self::INPUT_WIDTH)
                .selected_text(selected_text)
                .show_ui(ui, |ui| {
                    for option in options {
                        let text = if option.is_empty() { "无" } else { option };
                        ui.selectable_value(&mut param.value, (*option).to_string(), text);
                    }
                });
        });
    }

    fn fingerprint_ui(ui: &mut egui::Ui, param: &mut Param, disabled: bool) {
        ui.horizontal(|ui| {
            ui.add_space(Self::CHECKBOX_WIDTH + ui.spacing().item_spacing.x);
            Self::right_aligned_label(ui, "fingerprint");

            let spacing = ui.spacing().item_spacing.x;
            let input_width = Self::INPUT_WIDTH - Self::RANDOM_BUTTON_WIDTH - spacing;
            ui.add_enabled_ui(!disabled, |ui| {
                ui.add_sized(
                    [input_width, 20.0],
                    egui::TextEdit::singleline(&mut param.value).hint_text("32位整数"),
                );
                if ui
                    .add_sized(
                        [Self::RANDOM_BUTTON_WIDTH, Self::ROW_HEIGHT],
                        egui::Button::new("随机"),
                    )
                    .clicked()
                {
                    let nanos = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|duration| duration.as_nanos())
                        .unwrap_or_default();
                    let seed = ((nanos >> 32) as u32) ^ (nanos as u32) ^ std::process::id();
                    param.value = seed.to_string();
                }
            });
        });
    }

    fn build_args(&self) -> Vec<String> {
        let c = &self.config;
        let mut args: Vec<String> = Vec::new();

        let mut path = c.user_data_dir.value.trim().to_string();
        if !c.fingerprint.value.trim().is_empty() {
            path.push_str("\\");
            path.push_str(c.fingerprint.value.trim());
        }
        Self::push_value_arg(&mut args, "user-data-dir", &path);

        Self::push_value_arg(&mut args, "fingerprint", &c.fingerprint.value);

        Self::push_value_arg(
            &mut args,
            "fingerprint-platform",
            &c.fingerprint_platform.value,
        );

        if c.fingerprint_platform_version.enabled {
            Self::push_value_arg(
                &mut args,
                "fingerprint-platform-version",
                &c.fingerprint_platform_version.value,
            );
        }

        Self::push_value_arg(&mut args, "fingerprint-brand", &c.fingerprint_brand.value);

        if c.fingerprint_brand_version.enabled {
            Self::push_value_arg(
                &mut args,
                "fingerprint-brand-version",
                &c.fingerprint_brand_version.value,
            );
        }

        if c.fingerprint_hardware_concurrency.enabled {
            Self::push_value_arg(
                &mut args,
                "fingerprint-hardware-concurrency",
                &c.fingerprint_hardware_concurrency.value,
            );
        }

        if c.disable_non_proxied_udp.enabled {
            args.push("--disable-non-proxied-udp".to_string());
        }

        if c.enable_cdp {
            Self::push_value_arg(&mut args, "remote-debugging-port", "9222");
        }

        Self::push_value_arg(&mut args, "lang", &c.lang.value);

        if c.accept_lang.enabled {
            Self::push_value_arg(&mut args, "accept-lang", &c.accept_lang.value);
        }

        Self::push_value_arg(&mut args, "timezone", &c.timezone.value);

        if c.proxy_server.enabled {
            Self::push_value_arg(&mut args, "proxy-server", &c.proxy_server.value);
        }

        if c.disable_spoofing.enabled {
            Self::push_value_arg(&mut args, "disable-spoofing", &c.disable_spoofing.value);
        }

        args
    }

    fn push_value_arg(args: &mut Vec<String>, name: &str, value: &str) {
        let value = value.trim();
        if !value.is_empty() {
            args.push(format!("--{}={}", name, value));
        }
    }

    fn validate_required_params(&mut self) -> bool {
        if self.config.user_data_dir.value.trim().is_empty() {
            self.status_message = "启动失败: user-data-dir 不能为空".to_string();
            self.status_color = egui::Color32::RED;
            return false;
        }

        if self.config.fingerprint.value.trim().is_empty() {
            self.status_message = "启动失败: fingerprint 不能为空".to_string();
            self.status_color = egui::Color32::RED;
            return false;
        }

        true
    }

    fn launch_chrome(&mut self, ctx: &egui::Context) {
        if self.config.auto_seed {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default();
            let seed = ((nanos >> 32) as u32) ^ (nanos as u32) ^ std::process::id();
            self.config.fingerprint.value = seed.to_string();
        }
        if !self.validate_required_params() {
            return;
        }

        let exe = std::env::current_dir()
            .unwrap_or_default()
            .join("chrome.exe");
        if !exe.exists() {
            self.status_message = format!("未找到: {}", exe.display());
            self.status_color = egui::Color32::RED;
            return;
        }

        let args = self.build_args();
        let mut cmd = Command::new(&exe);
        cmd.args(&args);

        match cmd.spawn() {
            Ok(_) => {
                self.config.save();
                if self.config.close_after_launch {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                } else {
                    self.status_message = "浏览器已启动，配置已保存".to_string();
                    self.status_color = egui::Color32::GREEN;
                }
            }
            Err(e) => {
                self.status_message = format!("启动失败: {}", e);
                self.status_color = egui::Color32::RED;
            }
        }
    }
}

impl eframe::App for FpUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.group(|ui| {
                ui.label("常用参数");
                ui.add_space(4.0);

                Self::required_param_ui(
                    ui,
                    "user-data-dir",
                    &mut self.config.user_data_dir,
                    "必填，如 d:\\chrome",
                );
                Self::fingerprint_ui(ui, &mut self.config.fingerprint, self.config.auto_seed);
                Self::select_param_ui(
                    ui,
                    "fingerprint-platform",
                    &mut self.config.fingerprint_platform,
                    &["", "windows", "macos", "linux"],
                );
                Self::select_param_ui(
                    ui,
                    "lang",
                    &mut self.config.lang,
                    &["", "zh-CN", "zh-TW", "en-US", "en-GB", "ja-JP", "ko-KR"],
                );
                Self::select_param_ui(
                    ui,
                    "timezone",
                    &mut self.config.timezone,
                    &[
                        "",
                        "Asia/Shanghai",
                        "Asia/Tokyo",
                        "Asia/Seoul",
                        "America/New_York",
                        "America/Los_Angeles",
                        "Europe/London",
                        "Europe/Berlin",
                        "UTC",
                    ],
                );
                Self::param_ui(
                    ui,
                    "proxy-server",
                    &mut self.config.proxy_server,
                    "如 http://127.0.0.1:8080",
                );
            });

            ui.add_space(8.0);

            if ui
                .button(if self.show_advanced {
                    "隐藏非常用参数"
                } else {
                    "显示非常用参数"
                })
                .clicked()
            {
                self.show_advanced = !self.show_advanced;
                Self::set_window_height(ctx, self.show_advanced);
            }

            if self.show_advanced {
                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.label("非常用参数");
                    ui.add_space(4.0);

                    Self::param_ui(
                        ui,
                        "fingerprint-platform-version",
                        &mut self.config.fingerprint_platform_version,
                        "操作系统版本",
                    );
                    Self::select_param_ui(
                        ui,
                        "fingerprint-brand",
                        &mut self.config.fingerprint_brand,
                        &["", "Chrome", "Edge", "Opera", "Vivaldi"],
                    );
                    Self::param_ui(
                        ui,
                        "fingerprint-brand-version",
                        &mut self.config.fingerprint_brand_version,
                        "品牌版本号",
                    );
                    Self::param_ui(
                        ui,
                        "fingerprint-hardware-concurrency",
                        &mut self.config.fingerprint_hardware_concurrency,
                        "CPU核心数",
                    );
                    Self::param_ui(
                        ui,
                        "accept-lang",
                        &mut self.config.accept_lang,
                        "如 zh-CN,en-US",
                    );
                    Self::param_ui(
                        ui,
                        "disable-spoofing",
                        &mut self.config.disable_spoofing,
                        "font,audio,canvas,clientrects,gpu",
                    );
                    Self::param_ui(
                        ui,
                        "disable-non-proxied-udp",
                        &mut self.config.disable_non_proxied_udp,
                        "可选值",
                    );
                });
            }

            ui.add_space(14.0);

            let enter_pressed = ctx.input(|input| input.key_pressed(egui::Key::Enter));
            ui.horizontal(|ui| {
                let spacing = ui.spacing().item_spacing.x;
                let form_right =
                    Self::CHECKBOX_WIDTH + Self::LABEL_WIDTH + Self::INPUT_WIDTH + spacing * 2.0;
                let cdp_width = 70.0;
                let close_after_launch_width = 110.0;
                let auto_seed_width = 100.0;
                ui.add_space(
                    (form_right - auto_seed_width - cdp_width - close_after_launch_width - Self::LAUNCH_BUTTON_WIDTH - spacing * 1.5)
                        .max(0.0),
                );
                ui.add_sized(
                    [auto_seed_width, Self::ROW_HEIGHT],
                    egui::Checkbox::new(&mut self.config.auto_seed, "自动种子"),
                )
                .on_hover_text("自动生成随机种子");
                ui.add_sized(
                    [cdp_width, Self::ROW_HEIGHT],
                    egui::Checkbox::new(&mut self.config.enable_cdp, "启用 CDP"),
                )
                .on_hover_text("启用 Chrome DevTools Protocol 端口 9222");
                ui.add_sized(
                    [close_after_launch_width, Self::ROW_HEIGHT],
                    egui::Checkbox::new(&mut self.config.close_after_launch, "启动后关闭"),
                )
                .on_hover_text("启动浏览器成功后关闭启动器");
                let launch_clicked = ui
                    .add_sized(
                        [Self::LAUNCH_BUTTON_WIDTH, Self::ROW_HEIGHT],
                        egui::Button::new("启动浏览器"),
                    )
                    .clicked();
                if launch_clicked || enter_pressed {
                    self.launch_chrome(ctx);
                }
            });

            ui.add_space(8.0);
            if !self.status_message.is_empty() {
                ui.colored_label(self.status_color, &self.status_message);
            } else {
                ui.label(" ");
            }
        });
    }
}
