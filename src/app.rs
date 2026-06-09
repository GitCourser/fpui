use crate::config::{Config, Param};
use native_windows_gui as nwg;
use std::{
    cell::{Cell, RefCell},
    error::Error,
    fmt, mem,
    path::PathBuf,
    process::Command,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};
use winapi::{
    shared::{
        minwindef::{BOOL, DWORD, LPCVOID, LRESULT},
        windef::{HBRUSH, HDC, RECT},
        winerror::ERROR_SUCCESS,
    },
    um::{
        dwmapi::DwmSetWindowAttribute,
        uxtheme::SetWindowTheme,
        wingdi::{CreateSolidBrush, DeleteObject, RGB, SetBkColor, SetTextColor},
        winreg::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW},
        winuser::{
            FillRect, GetClientRect, RDW_ALLCHILDREN, RDW_ERASE, RDW_FRAME, RDW_INVALIDATE,
            RDW_UPDATENOW, RedrawWindow, WM_CTLCOLORBTN, WM_CTLCOLOREDIT, WM_CTLCOLORLISTBOX,
            WM_CTLCOLORSTATIC, WM_ERASEBKGND, WM_SETTINGCHANGE, WM_THEMECHANGED,
        },
    },
};

const APP_ICON_RESOURCE_ID: usize = 1;

const WINDOW_WIDTH: i32 = 615;
const COLLAPSED_HEIGHT: i32 = 310;
const EXPANDED_HEIGHT: i32 = 545;
const LABEL_WIDTH: i32 = 205;
const INPUT_WIDTH: i32 = 320;
const CHECK_X: i32 = 18;
const LABEL_X: i32 = 50;
const INPUT_X: i32 = 265;
const ROW_HEIGHT: i32 = 24;
const ROW_STEP: i32 = 28;
const RANDOM_BUTTON_WIDTH: i32 = 58;
const LAUNCH_BUTTON_WIDTH: i32 = 92;
const BOTTOM_AUTO_SEED_X: i32 = 192;
const BOTTOM_ENABLE_CDP_X: i32 = 291;
const BOTTOM_CLOSE_AFTER_LAUNCH_X: i32 = 397;
const BOTTOM_LAUNCH_BUTTON_X: i32 = 510;
const CHECK_LABEL_OFFSET: i32 = 23;

const PLATFORM_OPTIONS: &[ComboOption] = &[
    ComboOption::none(),
    ComboOption::new("windows", "windows"),
    ComboOption::new("macos", "macos"),
    ComboOption::new("linux", "linux"),
];
const LANG_OPTIONS: &[ComboOption] = &[
    ComboOption::none(),
    ComboOption::new("zh-CN", "zh-CN"),
    ComboOption::new("zh-TW", "zh-TW"),
    ComboOption::new("en-US", "en-US"),
    ComboOption::new("en-GB", "en-GB"),
    ComboOption::new("ja-JP", "ja-JP"),
    ComboOption::new("ko-KR", "ko-KR"),
];
const TIMEZONE_OPTIONS: &[ComboOption] = &[
    ComboOption::none(),
    ComboOption::new("Asia/Shanghai", "Asia/Shanghai"),
    ComboOption::new("Asia/Tokyo", "Asia/Tokyo"),
    ComboOption::new("Asia/Seoul", "Asia/Seoul"),
    ComboOption::new("America/New_York", "America/New_York"),
    ComboOption::new("America/Los_Angeles", "America/Los_Angeles"),
    ComboOption::new("Europe/London", "Europe/London"),
    ComboOption::new("Europe/Berlin", "Europe/Berlin"),
    ComboOption::new("UTC", "UTC"),
];
const BRAND_OPTIONS: &[ComboOption] = &[
    ComboOption::none(),
    ComboOption::new("Chrome", "Chrome"),
    ComboOption::new("Edge", "Edge"),
    ComboOption::new("Opera", "Opera"),
    ComboOption::new("Vivaldi", "Vivaldi"),
];

#[derive(Clone, Copy)]
struct ComboOption {
    value: &'static str,
    label: &'static str,
}

impl ComboOption {
    const fn new(value: &'static str, label: &'static str) -> Self {
        Self { value, label }
    }

    const fn none() -> Self {
        Self::new("", "无")
    }
}

impl Default for ComboOption {
    fn default() -> Self {
        Self::none()
    }
}

impl fmt::Display for ComboOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label)
    }
}

#[derive(Default)]
struct ParamControls {
    checkbox: Option<nwg::CheckBox>,
    label: nwg::Label,
    input: nwg::TextInput,
}

impl ParamControls {
    fn set_visible(&self, visible: bool) {
        if let Some(checkbox) = &self.checkbox {
            checkbox.set_visible(visible);
        }
        self.label.set_visible(visible);
        self.input.set_visible(visible);
    }

    fn read(&self, param: &mut Param) {
        if let Some(checkbox) = &self.checkbox {
            param.enabled = checkbox.check_state() == nwg::CheckBoxState::Checked;
        }
        param.value = self.input.text();
    }
}

#[derive(Default)]
struct ComboParamControls {
    label: nwg::Label,
    combo: nwg::ComboBox<ComboOption>,
    options: &'static [ComboOption],
}

impl ComboParamControls {
    fn set_visible(&self, visible: bool) {
        self.label.set_visible(visible);
        self.combo.set_visible(visible);
    }

    fn read(&self, param: &mut Param) {
        param.value = self
            .combo
            .selection()
            .and_then(|index| self.options.get(index))
            .map(|item| item.value.to_string())
            .unwrap_or_default();
    }
}

#[derive(Default)]
pub struct FpUiApp {
    config: RefCell<Config>,
    show_advanced: Cell<bool>,

    window: nwg::Window,
    icon: Option<nwg::Icon>,
    common_title: nwg::Label,
    advanced_title: nwg::Label,

    user_data_dir: ParamControls,
    fingerprint_label: nwg::Label,
    fingerprint_input: nwg::TextInput,
    random_button: nwg::Button,
    fingerprint_platform: ComboParamControls,
    lang: ComboParamControls,
    timezone: ComboParamControls,
    proxy_server: ParamControls,

    fingerprint_platform_version: ParamControls,
    fingerprint_brand: ComboParamControls,
    fingerprint_brand_version: ParamControls,
    fingerprint_hardware_concurrency: ParamControls,
    accept_lang: ParamControls,
    disable_spoofing: ParamControls,
    disable_non_proxied_udp: ParamControls,

    advanced_button: nwg::Button,
    auto_seed: nwg::CheckBox,
    auto_seed_label: nwg::Label,
    enable_cdp: nwg::CheckBox,
    enable_cdp_label: nwg::Label,
    close_after_launch: nwg::CheckBox,
    close_after_launch_label: nwg::Label,
    launch_button: nwg::Button,
    status_label: nwg::Label,

    theme_brushes: RefCell<Option<ThemeBrushes>>,
    event_handler: RefCell<Option<nwg::EventHandler>>,
    raw_event_handler: RefCell<Option<nwg::RawEventHandler>>,
}

impl FpUiApp {
    pub fn build() -> Result<Rc<Self>, Box<dyn Error>> {
        let mut app = Self::default();
        app.config = RefCell::new(Config::load());

        let icon = nwg::EmbedResource::load(None)
            .ok()
            .and_then(|embed| nwg::Icon::from_embed(&embed, Some(APP_ICON_RESOURCE_ID), None).ok());
        app.icon = icon;

        let theme = Theme::current();
        let window_title = format!("指纹浏览器启动器 v{}", env!("CARGO_PKG_VERSION"));
        nwg::Window::builder()
            .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::MINIMIZE_BOX)
            .size((WINDOW_WIDTH, COLLAPSED_HEIGHT))
            .center(true)
            .title(&window_title)
            .icon(app.icon.as_ref())
            .build(&mut app.window)?;

        build_section_title(
            &mut app.common_title,
            &app.window,
            "常用参数",
            24,
            10,
            theme,
        )?;

        {
            let cfg = app.config.borrow();
            build_required_param_row(
                &mut app.user_data_dir,
                &app.window,
                "user-data-dir",
                &cfg.user_data_dir,
                "必填，如 d:\\chrome",
                42,
                theme,
            )?;
            build_fingerprint_row(
                &mut app.fingerprint_label,
                &mut app.fingerprint_input,
                &mut app.random_button,
                &app.window,
                &cfg.fingerprint,
                cfg.auto_seed,
                42 + ROW_STEP,
                theme,
            )?;
            build_combo_row(
                &mut app.fingerprint_platform,
                &app.window,
                "fingerprint-platform",
                &cfg.fingerprint_platform,
                PLATFORM_OPTIONS,
                42 + ROW_STEP * 2,
                theme,
            )?;
            build_combo_row(
                &mut app.lang,
                &app.window,
                "lang",
                &cfg.lang,
                LANG_OPTIONS,
                42 + ROW_STEP * 3,
                theme,
            )?;
            build_combo_row(
                &mut app.timezone,
                &app.window,
                "timezone",
                &cfg.timezone,
                TIMEZONE_OPTIONS,
                42 + ROW_STEP * 4,
                theme,
            )?;
            build_optional_param_row(
                &mut app.proxy_server,
                &app.window,
                "proxy-server",
                &cfg.proxy_server,
                "如 http://127.0.0.1:8080",
                42 + ROW_STEP * 5,
                theme,
            )?;
        }

        build_section_title(
            &mut app.advanced_title,
            &app.window,
            "非常用参数",
            24,
            248,
            theme,
        )?;

        {
            let cfg = app.config.borrow();
            build_optional_param_row(
                &mut app.fingerprint_platform_version,
                &app.window,
                "fingerprint-platform-version",
                &cfg.fingerprint_platform_version,
                "操作系统版本",
                280,
                theme,
            )?;
            build_combo_row(
                &mut app.fingerprint_brand,
                &app.window,
                "fingerprint-brand",
                &cfg.fingerprint_brand,
                BRAND_OPTIONS,
                280 + ROW_STEP,
                theme,
            )?;
            build_optional_param_row(
                &mut app.fingerprint_brand_version,
                &app.window,
                "fingerprint-brand-version",
                &cfg.fingerprint_brand_version,
                "品牌版本号",
                280 + ROW_STEP * 2,
                theme,
            )?;
            build_optional_param_row(
                &mut app.fingerprint_hardware_concurrency,
                &app.window,
                "fingerprint-hardware-concurrency",
                &cfg.fingerprint_hardware_concurrency,
                "CPU核心数",
                280 + ROW_STEP * 3,
                theme,
            )?;
            build_optional_param_row(
                &mut app.accept_lang,
                &app.window,
                "accept-lang",
                &cfg.accept_lang,
                "如 zh-CN,en-US",
                280 + ROW_STEP * 4,
                theme,
            )?;
            build_optional_param_row(
                &mut app.disable_spoofing,
                &app.window,
                "disable-spoofing",
                &cfg.disable_spoofing,
                "font,audio,canvas,clientrects,gpu",
                280 + ROW_STEP * 5,
                theme,
            )?;
            build_optional_param_row(
                &mut app.disable_non_proxied_udp,
                &app.window,
                "disable-non-proxied-udp",
                &cfg.disable_non_proxied_udp,
                "可选值",
                280 + ROW_STEP * 6,
                theme,
            )?;
        }

        nwg::Button::builder()
            .text("显示非常用参数")
            .parent(&app.window)
            .position((18, 218))
            .size((125, ROW_HEIGHT))
            .build(&mut app.advanced_button)?;

        {
            let cfg = app.config.borrow();
            build_check_option(
                &mut app.auto_seed,
                &mut app.auto_seed_label,
                &app.window,
                "自动种子",
                cfg.auto_seed,
                (BOTTOM_AUTO_SEED_X, 252),
                (91, ROW_HEIGHT),
                theme,
            )?;
            build_check_option(
                &mut app.enable_cdp,
                &mut app.enable_cdp_label,
                &app.window,
                "启用 CDP",
                cfg.enable_cdp,
                (BOTTOM_ENABLE_CDP_X, 252),
                (98, ROW_HEIGHT),
                theme,
            )?;
            build_check_option(
                &mut app.close_after_launch,
                &mut app.close_after_launch_label,
                &app.window,
                "启动后关闭",
                cfg.close_after_launch,
                (BOTTOM_CLOSE_AFTER_LAUNCH_X, 252),
                (105, ROW_HEIGHT),
                theme,
            )?;
        }

        nwg::Button::builder()
            .text("启动浏览器")
            .parent(&app.window)
            .position((BOTTOM_LAUNCH_BUTTON_X, 252))
            .size((LAUNCH_BUTTON_WIDTH, ROW_HEIGHT))
            .build(&mut app.launch_button)?;

        build_label(
            &mut app.status_label,
            &app.window,
            " ",
            (18, 282),
            (570, ROW_HEIGHT),
            nwg::HTextAlign::Left,
            theme,
        )?;

        app.set_advanced_visible(false);

        let app = Rc::new(app);
        Self::bind_events(&app)?;
        app.apply_current_theme();
        app.window.set_visible(true);
        app.apply_current_theme();
        app.user_data_dir.input.set_focus();
        Ok(app)
    }

    fn bind_events(app: &Rc<Self>) -> Result<(), Box<dyn Error>> {
        let weak = Rc::downgrade(app);
        let handler =
            nwg::full_bind_event_handler(&app.window.handle, move |evt, _data, handle| {
                let Some(app) = weak.upgrade() else {
                    return;
                };

                match evt {
                    nwg::Event::OnWindowClose if handle == app.window => {
                        nwg::stop_thread_dispatch();
                    }
                    nwg::Event::OnButtonClick if handle == app.advanced_button => {
                        app.toggle_advanced();
                    }
                    nwg::Event::OnButtonClick if handle == app.random_button => {
                        app.randomize_fingerprint();
                    }
                    nwg::Event::OnButtonClick if handle == app.auto_seed => {
                        let auto_seed = app.is_checked(&app.auto_seed);
                        app.set_fingerprint_controls_enabled(!auto_seed);
                    }
                    nwg::Event::OnButtonClick if handle == app.launch_button => {
                        app.launch_chrome();
                    }
                    nwg::Event::OnKeyEnter => {
                        app.launch_chrome();
                    }
                    _ => {}
                }
            });
        *app.event_handler.borrow_mut() = Some(handler);

        let weak = Rc::downgrade(app);
        let raw_handler =
            nwg::bind_raw_event_handler(&app.window.handle, 0x10001, move |hwnd, msg, w, _l| {
                let app = weak.upgrade()?;

                if let Some(result) = app.handle_theme_paint(hwnd, msg, w) {
                    return Some(result);
                }

                let theme_changed = msg == WM_SETTINGCHANGE || msg == WM_THEMECHANGED;
                if theme_changed {
                    app.apply_current_theme();
                }
                None
            })?;
        *app.raw_event_handler.borrow_mut() = Some(raw_handler);

        Ok(())
    }

    fn set_advanced_visible(&self, visible: bool) {
        self.advanced_title.set_visible(visible);
        self.fingerprint_platform_version.set_visible(visible);
        self.fingerprint_brand.set_visible(visible);
        self.fingerprint_brand_version.set_visible(visible);
        self.fingerprint_hardware_concurrency.set_visible(visible);
        self.accept_lang.set_visible(visible);
        self.disable_spoofing.set_visible(visible);
        self.disable_non_proxied_udp.set_visible(visible);

        let bottom_y = if visible { 492 } else { 252 };
        let status_y = if visible { 522 } else { 282 };
        self.auto_seed.set_position(BOTTOM_AUTO_SEED_X, bottom_y);
        self.auto_seed_label
            .set_position(BOTTOM_AUTO_SEED_X + CHECK_LABEL_OFFSET, bottom_y);
        self.enable_cdp.set_position(BOTTOM_ENABLE_CDP_X, bottom_y);
        self.enable_cdp_label
            .set_position(BOTTOM_ENABLE_CDP_X + CHECK_LABEL_OFFSET, bottom_y);
        self.close_after_launch
            .set_position(BOTTOM_CLOSE_AFTER_LAUNCH_X, bottom_y);
        self.close_after_launch_label
            .set_position(BOTTOM_CLOSE_AFTER_LAUNCH_X + CHECK_LABEL_OFFSET, bottom_y);
        self.launch_button
            .set_position(BOTTOM_LAUNCH_BUTTON_X, bottom_y);
        self.status_label.set_position(18, status_y);
        self.window.set_size(
            WINDOW_WIDTH as u32,
            if visible {
                EXPANDED_HEIGHT as u32
            } else {
                COLLAPSED_HEIGHT as u32
            },
        );
        self.advanced_button.set_text(if visible {
            "隐藏非常用参数"
        } else {
            "显示非常用参数"
        });
        self.window.invalidate();
    }

    fn toggle_advanced(&self) {
        let visible = !self.show_advanced.get();
        self.show_advanced.set(visible);
        self.set_advanced_visible(visible);
    }

    fn apply_current_theme(&self) {
        let theme = Theme::current();
        set_window_dark_mode(&self.window, theme.dark);
        self.apply_combo_themes(theme.dark);
        *self.theme_brushes.borrow_mut() = Some(ThemeBrushes::new(theme));
        redraw_window(&self.window);
    }

    fn apply_combo_themes(&self, dark: bool) {
        set_combo_dark_theme(&self.fingerprint_platform.combo, dark);
        set_combo_dark_theme(&self.lang.combo, dark);
        set_combo_dark_theme(&self.timezone.combo, dark);
        set_combo_dark_theme(&self.fingerprint_brand.combo, dark);
    }

    fn handle_theme_paint(
        &self,
        hwnd: winapi::shared::windef::HWND,
        msg: u32,
        w: usize,
    ) -> Option<LRESULT> {
        let brushes = self.theme_brushes.borrow();
        let brushes = brushes.as_ref()?;

        match msg {
            WM_ERASEBKGND => {
                let hdc = w as HDC;
                let mut rect: RECT = unsafe { mem::zeroed() };
                unsafe {
                    GetClientRect(hwnd, &mut rect);
                    FillRect(hdc, &rect, brushes.window);
                }
                draw_section_frames(hdc, brushes, self.show_advanced.get());
                Some(1)
            }
            WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
                apply_control_colors(w as HDC, brushes.theme.text, brushes.theme.background);
                Some(brushes.window as LRESULT)
            }
            WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
                apply_control_colors(w as HDC, brushes.theme.text, brushes.theme.edit_background);
                Some(brushes.edit as LRESULT)
            }
            _ => None,
        }
    }

    fn randomize_fingerprint(&self) {
        self.fingerprint_input.set_text(&random_seed());
    }

    fn set_fingerprint_controls_enabled(&self, enabled: bool) {
        self.fingerprint_input.set_enabled(enabled);
        self.random_button.set_enabled(enabled);
    }

    fn is_checked(&self, checkbox: &nwg::CheckBox) -> bool {
        checkbox.check_state() == nwg::CheckBoxState::Checked
    }

    fn sync_config_from_controls(&self) {
        let mut config = self.config.borrow_mut();
        self.user_data_dir.read(&mut config.user_data_dir);
        config.fingerprint.value = self.fingerprint_input.text();
        self.fingerprint_platform
            .read(&mut config.fingerprint_platform);
        self.lang.read(&mut config.lang);
        self.timezone.read(&mut config.timezone);
        self.proxy_server.read(&mut config.proxy_server);

        self.fingerprint_platform_version
            .read(&mut config.fingerprint_platform_version);
        self.fingerprint_brand.read(&mut config.fingerprint_brand);
        self.fingerprint_brand_version
            .read(&mut config.fingerprint_brand_version);
        self.fingerprint_hardware_concurrency
            .read(&mut config.fingerprint_hardware_concurrency);
        self.accept_lang.read(&mut config.accept_lang);
        self.disable_spoofing.read(&mut config.disable_spoofing);
        self.disable_non_proxied_udp
            .read(&mut config.disable_non_proxied_udp);

        config.auto_seed = self.is_checked(&self.auto_seed);
        config.enable_cdp = self.is_checked(&self.enable_cdp);
        config.close_after_launch = self.is_checked(&self.close_after_launch);
    }

    fn validate_required_params(&self) -> bool {
        let config = self.config.borrow();
        if config.user_data_dir.value.trim().is_empty() {
            self.set_status("启动失败: user-data-dir 不能为空");
            return false;
        }

        if config.fingerprint.value.trim().is_empty() {
            self.set_status("启动失败: fingerprint 不能为空");
            return false;
        }

        true
    }

    fn launch_chrome(&self) {
        self.sync_config_from_controls();

        if self.config.borrow().auto_seed {
            let seed = random_seed();
            self.fingerprint_input.set_text(&seed);
            self.config.borrow_mut().fingerprint.value = seed;
        }

        if !self.validate_required_params() {
            return;
        }

        let exe = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("chrome.exe");
        if !exe.exists() {
            self.set_status(&format!("未找到: {}", exe.display()));
            return;
        }

        let args = self.build_args();
        let mut cmd = Command::new(&exe);
        cmd.args(&args);

        match cmd.spawn() {
            Ok(_) => {
                self.config.borrow().save();
                if self.config.borrow().close_after_launch {
                    self.window.close();
                } else {
                    self.set_status("浏览器已启动，配置已保存");
                }
            }
            Err(err) => {
                self.set_status(&format!("启动失败: {}", err));
            }
        }
    }

    fn build_args(&self) -> Vec<String> {
        let c = self.config.borrow();
        let mut args: Vec<String> = Vec::new();

        let mut path = c.user_data_dir.value.trim().to_string();
        if !c.fingerprint.value.trim().is_empty() {
            path.push('\\');
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

    fn set_status(&self, message: &str) {
        self.status_label.set_text(message);
    }
}

impl Drop for FpUiApp {
    fn drop(&mut self) {
        if let Some(handler) = self.raw_event_handler.borrow().as_ref() {
            let _ = nwg::unbind_raw_event_handler(handler);
        }
        if let Some(handler) = self.event_handler.borrow().as_ref() {
            nwg::unbind_event_handler(handler);
        }
    }
}

#[derive(Clone, Copy)]
struct Theme {
    dark: bool,
    background: [u8; 3],
    edit_background: [u8; 3],
    text: [u8; 3],
    border: [u8; 3],
}

impl Theme {
    fn current() -> Self {
        let dark = system_prefers_dark_mode();
        if dark {
            Self {
                dark,
                background: [32, 32, 32],
                edit_background: [45, 45, 45],
                text: [230, 230, 230],
                border: [58, 58, 58],
            }
        } else {
            Self {
                dark,
                background: [240, 240, 240],
                edit_background: [255, 255, 255],
                text: [0, 0, 0],
                border: [160, 160, 160],
            }
        }
    }
}

struct ThemeBrushes {
    theme: Theme,
    window: HBRUSH,
    edit: HBRUSH,
    border: HBRUSH,
}

impl ThemeBrushes {
    fn new(theme: Theme) -> Self {
        Self {
            theme,
            window: solid_brush(theme.background),
            edit: solid_brush(theme.edit_background),
            border: solid_brush(theme.border),
        }
    }
}

impl Drop for ThemeBrushes {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.window as _);
            let _ = DeleteObject(self.edit as _);
            let _ = DeleteObject(self.border as _);
        }
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    nwg::init()?;
    set_global_ui_font();

    let _app = FpUiApp::build()?;
    nwg::dispatch_thread_events();
    Ok(())
}

fn build_section_title(
    label: &mut nwg::Label,
    parent: &nwg::Window,
    text: &str,
    x: i32,
    y: i32,
    theme: Theme,
) -> Result<(), nwg::NwgError> {
    build_label(
        label,
        parent,
        text,
        (x, y),
        (110, ROW_HEIGHT),
        nwg::HTextAlign::Left,
        theme,
    )
}

fn build_required_param_row(
    controls: &mut ParamControls,
    parent: &nwg::Window,
    label: &str,
    param: &Param,
    hint: &str,
    y: i32,
    theme: Theme,
) -> Result<(), nwg::NwgError> {
    build_label(
        &mut controls.label,
        parent,
        label,
        (LABEL_X, y),
        (LABEL_WIDTH, ROW_HEIGHT),
        nwg::HTextAlign::Right,
        theme,
    )?;
    build_text_input(
        &mut controls.input,
        parent,
        &param.value,
        hint,
        (INPUT_X, y),
        (INPUT_WIDTH, ROW_HEIGHT),
        theme,
    )?;
    Ok(())
}

fn build_optional_param_row(
    controls: &mut ParamControls,
    parent: &nwg::Window,
    label: &str,
    param: &Param,
    hint: &str,
    y: i32,
    theme: Theme,
) -> Result<(), nwg::NwgError> {
    let mut checkbox = nwg::CheckBox::default();
    build_check_box(
        &mut checkbox,
        parent,
        "",
        param.enabled,
        (CHECK_X, y),
        (24, ROW_HEIGHT),
        theme,
    )?;
    controls.checkbox = Some(checkbox);
    build_required_param_row(controls, parent, label, param, hint, y, theme)
}

#[allow(clippy::too_many_arguments)]
fn build_fingerprint_row(
    label: &mut nwg::Label,
    input: &mut nwg::TextInput,
    button: &mut nwg::Button,
    parent: &nwg::Window,
    param: &Param,
    auto_seed: bool,
    y: i32,
    theme: Theme,
) -> Result<(), nwg::NwgError> {
    build_label(
        label,
        parent,
        "fingerprint",
        (LABEL_X, y),
        (LABEL_WIDTH, ROW_HEIGHT),
        nwg::HTextAlign::Right,
        theme,
    )?;
    let input_width = INPUT_WIDTH - RANDOM_BUTTON_WIDTH - 8;
    build_text_input(
        input,
        parent,
        &param.value,
        "32位整数",
        (INPUT_X, y),
        (input_width, ROW_HEIGHT),
        theme,
    )?;
    input.set_enabled(!auto_seed);

    nwg::Button::builder()
        .text("随机")
        .parent(parent)
        .position((INPUT_X + input_width + 8, y))
        .size((RANDOM_BUTTON_WIDTH, ROW_HEIGHT))
        .enabled(!auto_seed)
        .build(button)
}

fn build_combo_row(
    controls: &mut ComboParamControls,
    parent: &nwg::Window,
    label: &str,
    param: &Param,
    options: &'static [ComboOption],
    y: i32,
    theme: Theme,
) -> Result<(), nwg::NwgError> {
    controls.options = options;
    build_label(
        &mut controls.label,
        parent,
        label,
        (LABEL_X, y),
        (LABEL_WIDTH, ROW_HEIGHT),
        nwg::HTextAlign::Right,
        theme,
    )?;

    let selected = options
        .iter()
        .position(|item| item.value == param.value.trim())
        .or(Some(0));

    nwg::ComboBox::builder()
        .parent(parent)
        .collection(options.to_vec())
        .selected_index(selected)
        .position((INPUT_X, y))
        .size((INPUT_WIDTH, 140))
        .build(&mut controls.combo)
}

fn build_label(
    label: &mut nwg::Label,
    parent: &nwg::Window,
    text: &str,
    position: (i32, i32),
    size: (i32, i32),
    align: nwg::HTextAlign,
    theme: Theme,
) -> Result<(), nwg::NwgError> {
    nwg::Label::builder()
        .text(text)
        .parent(parent)
        .position(position)
        .size(size)
        .h_align(align)
        .background_color(Some(theme.background))
        .build(label)
}

fn build_text_input(
    input: &mut nwg::TextInput,
    parent: &nwg::Window,
    text: &str,
    hint: &str,
    position: (i32, i32),
    size: (i32, i32),
    theme: Theme,
) -> Result<(), nwg::NwgError> {
    nwg::TextInput::builder()
        .text(text)
        .placeholder_text(Some(hint))
        .parent(parent)
        .position(position)
        .size(size)
        .background_color(Some(theme.background))
        .build(input)
}

fn build_check_box(
    checkbox: &mut nwg::CheckBox,
    parent: &nwg::Window,
    text: &str,
    checked: bool,
    position: (i32, i32),
    size: (i32, i32),
    theme: Theme,
) -> Result<(), nwg::NwgError> {
    nwg::CheckBox::builder()
        .text(text)
        .parent(parent)
        .position(position)
        .size(size)
        .check_state(if checked {
            nwg::CheckBoxState::Checked
        } else {
            nwg::CheckBoxState::Unchecked
        })
        .background_color(Some(theme.background))
        .build(checkbox)
}

#[allow(clippy::too_many_arguments)]
fn build_check_option(
    checkbox: &mut nwg::CheckBox,
    label: &mut nwg::Label,
    parent: &nwg::Window,
    text: &str,
    checked: bool,
    position: (i32, i32),
    size: (i32, i32),
    theme: Theme,
) -> Result<(), nwg::NwgError> {
    build_check_box(checkbox, parent, "", checked, position, (20, size.1), theme)?;
    build_label(
        label,
        parent,
        text,
        (position.0 + 23, position.1),
        (size.0 - 23, size.1),
        nwg::HTextAlign::Left,
        theme,
    )
}

fn random_seed() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let seed = ((nanos >> 32) as u32) ^ (nanos as u32) ^ std::process::id();
    seed.to_string()
}

fn set_global_ui_font() {
    for family in ["Microsoft YaHei UI", "Microsoft YaHei", "Segoe UI"] {
        let mut font = nwg::Font::default();
        if nwg::Font::builder()
            .family(family)
            .size_absolute(14)
            .build(&mut font)
            .is_ok()
        {
            nwg::Font::set_global_default(Some(font));
            return;
        }
    }
}

fn draw_section_frames(hdc: HDC, brushes: &ThemeBrushes, show_advanced: bool) {
    let common = RECT {
        left: 12,
        top: 14,
        right: 602,
        bottom: 214,
    };
    draw_section_frame(hdc, brushes.border, common, (20, 140));

    if show_advanced {
        let advanced = RECT {
            left: 12,
            top: 252,
            right: 602,
            bottom: 480,
        };
        draw_section_frame(hdc, brushes.border, advanced, (20, 140));
    }
}

fn draw_section_frame(hdc: HDC, brush: HBRUSH, rect: RECT, title_gap: (i32, i32)) {
    let top_left = RECT {
        left: rect.left,
        top: rect.top,
        right: title_gap.0,
        bottom: rect.top + 1,
    };
    let top_right = RECT {
        left: title_gap.1,
        top: rect.top,
        right: rect.right,
        bottom: rect.top + 1,
    };
    let left = RECT {
        left: rect.left,
        top: rect.top,
        right: rect.left + 1,
        bottom: rect.bottom,
    };
    let right = RECT {
        left: rect.right - 1,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    };
    let bottom = RECT {
        left: rect.left,
        top: rect.bottom - 1,
        right: rect.right,
        bottom: rect.bottom,
    };

    unsafe {
        FillRect(hdc, &top_left, brush);
        FillRect(hdc, &top_right, brush);
        FillRect(hdc, &left, brush);
        FillRect(hdc, &right, brush);
        FillRect(hdc, &bottom, brush);
    }
}

fn solid_brush(color: [u8; 3]) -> HBRUSH {
    unsafe { CreateSolidBrush(RGB(color[0], color[1], color[2])) }
}

fn color_ref(color: [u8; 3]) -> DWORD {
    RGB(color[0], color[1], color[2])
}

fn apply_control_colors(hdc: HDC, text: [u8; 3], background: [u8; 3]) {
    unsafe {
        SetTextColor(hdc, color_ref(text));
        SetBkColor(hdc, color_ref(background));
    }
}

fn system_prefers_dark_mode() -> bool {
    let path = wide_null("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let value_name = wide_null("AppsUseLightTheme");
    let mut value: DWORD = 1;
    let mut value_size = std::mem::size_of::<DWORD>() as DWORD;

    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            path.as_ptr(),
            value_name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            &mut value as *mut DWORD as *mut _,
            &mut value_size,
        )
    };

    status == ERROR_SUCCESS as i32 && value == 0
}

fn set_window_dark_mode(window: &nwg::Window, dark: bool) {
    let Some(hwnd) = window.handle.hwnd() else {
        return;
    };

    let enabled: BOOL = if dark { 1 } else { 0 };
    let size = std::mem::size_of::<BOOL>() as DWORD;

    unsafe {
        let _ = DwmSetWindowAttribute(hwnd, 20, &enabled as *const BOOL as LPCVOID, size);
        let _ = DwmSetWindowAttribute(hwnd, 19, &enabled as *const BOOL as LPCVOID, size);
    }
}

fn set_combo_dark_theme(combo: &nwg::ComboBox<ComboOption>, dark: bool) {
    let Some(hwnd) = combo.handle.hwnd() else {
        return;
    };

    unsafe {
        if dark {
            let dark_mode = wide_null("DarkMode_CFD");
            let _ = SetWindowTheme(hwnd, dark_mode.as_ptr(), std::ptr::null());
        } else {
            let _ = SetWindowTheme(hwnd, std::ptr::null(), std::ptr::null());
        }
    }
}

fn redraw_window(window: &nwg::Window) {
    let Some(hwnd) = window.handle.hwnd() else {
        return;
    };

    unsafe {
        RedrawWindow(
            hwnd,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            RDW_INVALIDATE | RDW_ERASE | RDW_FRAME | RDW_ALLCHILDREN | RDW_UPDATENOW,
        );
    }
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
