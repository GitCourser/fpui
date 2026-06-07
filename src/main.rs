#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod app;
#[cfg(windows)]
mod config;

#[cfg(windows)]
fn main() {
    if let Err(err) = app::run() {
        native_windows_gui::modal_error_message(
            &native_windows_gui::Window::default(),
            "启动失败",
            &err.to_string(),
        );
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("fpui 已迁移为 native-windows-gui，仅支持 Windows/MSVC 目标编译运行");
}
