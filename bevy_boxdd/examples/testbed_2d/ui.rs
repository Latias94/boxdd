use crate::control::{EventStats, TestbedState};
use crate::scenes::{SCENE_REGISTRY, TestbedEntity};
use crate::switch_scene;
use bevy::prelude::*;
use bevy_egui::egui::{LayerId, Ui, UiBuilder};
use bevy_egui::{EguiContexts, egui};

pub(crate) fn draw_testbed_ui(
    mut contexts: EguiContexts,
    mut state: ResMut<TestbedState>,
    mut commands: Commands,
    entities: Query<Entity, With<TestbedEntity>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut stats: ResMut<EventStats>,
) -> Result {
    let mut requested_scene = None;

    let ctx = contexts.ctx_mut()?;
    let mut root_ui = Ui::new(
        ctx.clone(),
        "boxdd_testbed_root".into(),
        UiBuilder::new()
            .layer_id(LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );

    egui::Panel::left("boxdd_testbed_panel")
        .default_size(300.0)
        .min_size(260.0)
        .resizable(true)
        .show(&mut root_ui, |ui| {
            ui.heading(if state.scene_switching_enabled {
                "boxdd Testbed"
            } else {
                SCENE_REGISTRY[state.scene_index].name
            });
            ui.separator();

            ui.horizontal(|ui| {
                if ui
                    .button(if state.paused { "Resume" } else { "Pause" })
                    .clicked()
                {
                    state.paused = !state.paused;
                    if !state.paused {
                        state.cancel_single_step();
                    }
                }
                if ui.button("Reset").clicked() {
                    requested_scene = Some(state.scene_index);
                }
                if ui
                    .add_enabled(state.paused, egui::Button::new("Step"))
                    .clicked()
                {
                    state.request_single_step();
                }
            });

            ui.separator();
            draw_scene_list(ui, &mut state, &mut requested_scene);

            ui.separator();
            ui.label(egui::RichText::new("World").strong());
            ui.checkbox(&mut state.gravity_enabled, "Gravity");
            ui.checkbox(&mut state.sleeping_enabled, "Sleeping");
            ui.checkbox(&mut state.warm_starting_enabled, "Warm starting");
            ui.checkbox(&mut state.continuous_enabled, "Continuous collision");
            ui.add(
                egui::Slider::new(
                    &mut state.sub_step_count,
                    crate::control::MIN_SUB_STEPS..=crate::control::MAX_SUB_STEPS,
                )
                .text("Substeps"),
            );
            ui.add(
                egui::Slider::new(
                    &mut state.hertz,
                    crate::control::MIN_HERTZ..=crate::control::MAX_HERTZ,
                )
                .text("Hz"),
            );

            ui.separator();
            ui.label(egui::RichText::new("Events").strong());
            ui.label(format!(
                "Contact begin/end: {} / {}",
                stats.contact_begin_total, stats.contact_end_total
            ));
            ui.label(format!("Contact hits: {}", stats.contact_hit_total));
            ui.label(format!(
                "Sensor begin/end: {} / {}",
                stats.sensor_begin_total, stats.sensor_end_total
            ));
            ui.label(format!(
                "This frame: C {} E {} H {} S {} X {}",
                stats.contact_begin_frame,
                stats.contact_end_frame,
                stats.contact_hit_frame,
                stats.sensor_begin_frame,
                stats.sensor_end_frame
            ));

            ui.separator();
            ui.label(egui::RichText::new("Draw").strong());
            ui.checkbox(&mut state.draw_overlays, "Joint overlays");
        });

    if let Some(scene_index) = requested_scene {
        switch_scene(
            scene_index,
            &mut state,
            &mut commands,
            &entities,
            &mut meshes,
            &mut materials,
            &mut stats,
        );
    }

    Ok(())
}

fn draw_scene_list(ui: &mut Ui, state: &mut TestbedState, requested_scene: &mut Option<usize>) {
    if state.scene_switching_enabled {
        ui.label(egui::RichText::new("Scenes").strong());
        egui::ScrollArea::vertical()
            .max_height(310.0)
            .show(ui, |ui| {
                let mut current_category = None;
                for (index, metadata) in SCENE_REGISTRY.iter().enumerate() {
                    if current_category != Some(metadata.category) {
                        current_category = Some(metadata.category);
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(metadata.category)
                                .small()
                                .color(egui::Color32::GRAY),
                        );
                    }

                    if ui
                        .selectable_label(state.scene_index == index, metadata.name)
                        .clicked()
                    {
                        *requested_scene = Some(index);
                    }

                    if state.scene_index == index {
                        draw_scene_metadata(ui, metadata);
                    }
                }
            });
    } else {
        draw_scene_metadata(ui, &SCENE_REGISTRY[state.scene_index]);
    }
}

fn draw_scene_metadata(ui: &mut Ui, metadata: &crate::scenes::TestbedSceneMetadata) {
    ui.label(egui::RichText::new(metadata.description).small());
    ui.label(
        egui::RichText::new(metadata.source_label())
            .small()
            .color(egui::Color32::LIGHT_GRAY),
    );
    for upstream in metadata.upstream {
        ui.label(
            egui::RichText::new(format!(
                "{} / {} ({})",
                upstream.category,
                upstream.name,
                upstream.mode.as_str()
            ))
            .small()
            .color(egui::Color32::GRAY),
        );
    }
}
