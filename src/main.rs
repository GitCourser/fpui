#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod config;

use app::FpUiApp;
use eframe::egui::IconData;
use egui_wgpu::wgpu;
use egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};
use std::sync::Arc;

const APP_ICON_BYTES: &[u8] = include_bytes!("../assets/ico.ico");

fn load_app_icon() -> Option<IconData> {
    let image = image::load_from_memory(APP_ICON_BYTES).ok()?.into_rgba8();
    let (width, height) = image.dimensions();

    Some(IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result {
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([615.0, 310.0])
        .with_min_inner_size([615.0, 310.0])
        .with_max_inner_size([615.0, 545.0])
        .with_resizable(false)
        .with_maximize_button(false);

    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        wgpu_options: WgpuConfiguration {
            wgpu_setup: WgpuSetup::CreateNew(WgpuSetupCreateNew {
                instance_descriptor: wgpu::InstanceDescriptor {
                    backends: wgpu::Backends::DX12 | wgpu::Backends::VULKAN | wgpu::Backends::GL,
                    flags: wgpu::InstanceFlags::default(),
                    backend_options: wgpu::BackendOptions::default(),
                },
                power_preference: wgpu::PowerPreference::None,
                native_adapter_selector: Some(Arc::new(|adapters, _surface| {
                    adapters
                        .iter()
                        .find(|adapter| {
                            matches!(
                                adapter.get_info().device_type,
                                wgpu::DeviceType::IntegratedGpu
                                    | wgpu::DeviceType::DiscreteGpu
                                    | wgpu::DeviceType::VirtualGpu
                                    | wgpu::DeviceType::Cpu
                                    | wgpu::DeviceType::Other
                            )
                        })
                        .cloned()
                        .ok_or_else(|| "没有可用的 wgpu 图形适配器".to_string())
                })),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };

    eframe::run_native(
        "指纹浏览器启动器",
        options,
        Box::new(|cc| Ok(Box::new(FpUiApp::new(cc)))),
    )
}
