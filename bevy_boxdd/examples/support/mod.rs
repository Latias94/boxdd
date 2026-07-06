use bevy::{app::PluginGroup, prelude::*};

#[cfg(target_os = "windows")]
use bevy::render::{
    RenderPlugin,
    settings::{Backends, WgpuSettings},
};

pub fn teaching_default_plugins(title: &'static str) -> impl PluginGroup {
    let window_plugin = WindowPlugin {
        primary_window: Some(teaching_window(title)),
        ..default()
    };

    #[cfg(target_os = "windows")]
    {
        DefaultPlugins.set(window_plugin).set(RenderPlugin {
            render_creation: WgpuSettings {
                backends: Some(Backends::from_env().unwrap_or(Backends::DX12)),
                ..default()
            }
            .into(),
            ..default()
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        DefaultPlugins.set(window_plugin)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn teaching_window(title: &'static str) -> Window {
    Window {
        title: title.into(),
        resolution: (1280, 800).into(),
        ..default()
    }
}

#[cfg(target_arch = "wasm32")]
fn teaching_window(title: &'static str) -> Window {
    Window {
        title: title.into(),
        canvas: Some("#bevy-canvas".into()),
        fit_canvas_to_parent: true,
        prevent_default_event_handling: false,
        ..default()
    }
}
