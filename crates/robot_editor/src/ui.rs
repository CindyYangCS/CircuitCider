use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    io::ErrorKind,
    thread::spawn,
};

use crate::{
    model_display::{components::DisplayModel, systems::display_model},
    shaders::neon_glow::NeonGlowMaterial,
};
use crate::{
    raycast_utils::{resources::MouseOverWindow, systems::*},
    resources::BuildToolMode,
};
use bevy::{
    asset::{AssetContainer, LoadedFolder},
    ecs::query::{QueryData, QueryFilter, ReadOnlyQueryData, WorldQuery},
    input::mouse::MouseButtonInput,
    log::tracing_subscriber::field::display,
    prelude::*,
    reflect::erased_serde::Error,
    render::{render_asset::RenderAssets, render_resource::TextureFormat, view::RenderLayers},
    window::PrimaryWindow,
};
use bevy_egui::EguiContext;
use bevy_mod_raycast::{
    immediate::{Raycast, RaycastSettings, RaycastVisibility},
    primitives::IntersectionData,
    CursorRay,
};
use bevy_rapier3d::{
    geometry::{Collider, Sensor},
    plugin::RapierContext,
    rapier::geometry::CollisionEventFlags,
};
use bevy_serialization_extras::prelude::{colliders::ColliderFlag, link::StructureFlag};
use egui::Align2;
use std::hash::Hash;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

use std::fmt::Debug;

#[derive(Resource, Default, Deref)]
pub struct ModelFolder(pub Handle<LoadedFolder>);

pub fn cache_initial_folders(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(ModelFolder(asset_server.load_folder("root://editor_model_parts")));
}

/// entity used to place other similar entities.
#[derive(Component, Default, Display)]
pub enum Placer {
    #[default]
    Hull,
    Wheel,
}

impl Placer {
    /// infer placer type from path
    pub fn from_path(path: &str) -> Self{
        let lower_case = path.to_lowercase();
        let split_path = lower_case.split(&['/', '.']).collect::<Vec<_>>();
        
        // println!("split path of placer is {:#?}", split_path);
        if split_path.contains(&"wheel") {
            Self::Wheel
        }
        else if split_path.contains(&"hull") {
            Self::Hull
        } 
        // default to hull if no valid placer type if found
        else {
            info!("cannot infer placer type from path. Defaulting to hull");
            Self::Hull
        }
        
    }
}

/// gets first raycast hit on entity with select component marker
// pub fn first_hit_with::<T: Component> {

// }

// #[derive(Debug)]
// pub struct GizmoMode {}

// impl Tool for GizmoMode {}

// pub trait Tool: Send + Sync + Debug {}

// #[derive(Resource, Debug)]
// pub struct ToolMode {
//     pub tool: Box<dyn Tool>,
// }

pub fn select_build_tool(
    mut primary_window: Query<&mut EguiContext, With<PrimaryWindow>>,
    mut tool_mode: ResMut<BuildToolMode>,
) {
    for mut context in primary_window.iter_mut() {
        egui::Window::new("BuildToolMode debug").show(context.get_mut(), |ui| {
            ui.heading("select mode");
            ui.label(format!("Current mode: {:#?}", *tool_mode));
            for tool in BuildToolMode::iter() {
                if ui.button(tool.to_string()).clicked() {
                    *tool_mode = tool
                }
            }
        });
    }
}

/// Sets mouse over window resource to true/false depending on mouse state.
pub fn check_if_mouse_over_ui(
    mut windows: Query<&mut EguiContext>,
    mut mouse_over_window: ResMut<MouseOverWindow>,
) {
    for mut window in windows.iter_mut() {
        if window.get_mut().is_pointer_over_area() {
            //println!("mouse is over window");
            **mouse_over_window = true
        } else {
            **mouse_over_window = false
        }
    }
    //**mouse_over_window = false
}

#[derive(Component, Default)]
pub struct Edited;

/// marker for objects that are not yet a part of a structure but could be
/// (placed build mode models)
#[derive(Component, Default)]
pub struct AttachCandidate;

// /// editor mode for editing attached
// pub fn editor_mode_ui

pub fn save_load_model_ui(
    mut primary_window: Query<&mut EguiContext, With<PrimaryWindow>>,
    //mut commands: Commands,
) {
    for mut context in primary_window.iter_mut() {
        let ui_name = "Save Load Model";
        egui::Window::new(ui_name)
            .anchor(Align2::RIGHT_TOP, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .show(context.get_mut(), |ui| {
                ui.label("save conditions");

                ui.horizontal(|ui| {
                    ui.button("save");
                    //ui.button("load");
                });
            });
    }
}

// #[derive(Resource, Deref, Default)]
// pub struct DisplayModelImage(pub Handle<Image>);


/// ui for editing functionality of placed part
pub fn placer_editor_ui(
    placers: Query<(&Placer, &Name)>,
    mut primary_window: Query<(&Window, &mut EguiContext), With<PrimaryWindow>>,
    keys: Res<ButtonInput<KeyCode>>

) {
    if placers.iter().len() <= 0 {return}

    for (win, mut context) in primary_window.iter_mut() {
        let ui_name = "Model features";

        let Some(cursor_pos) = win.cursor_position() else {return};

        // offset cursor pos to not have mouse click on this window
        let offset_cursor_pos = Vec2::new(cursor_pos.x + 10.0, cursor_pos.y - 10.0);
        let mut window = egui::Window::new(ui_name);
        
        // have window follow cursor if not kept in place
        if keys.pressed(KeyCode::ControlLeft) == false {
            window = window.fixed_pos(offset_cursor_pos.to_array());
        }
        
        window
        //.
        .show(context.get_mut(), |ui| {
            ui.label("text");
            for (placer, name) in placers.iter() {
                ui.label(format!("name: {:#}", name.to_string()));
            
                ui.label(format!("Placer type: {:#?}", placer.to_string()));
            }
        })
        ;
        
    }
}

/// list all placeable models
pub fn placer_spawner_ui(
    folders: Res<Assets<LoadedFolder>>,
    model_folder: Res<ModelFolder>,
    mut tool_mode: ResMut<BuildToolMode>,
    mut placer_materials: ResMut<Assets<NeonGlowMaterial>>,
    mut primary_window: Query<&mut EguiContext, With<PrimaryWindow>>,
    display_models: Query<(Entity, &Handle<Mesh>), With<DisplayModel>>,

    mut commands: Commands,
) {
    //if tool_mode.into_inner() == &BuildToolMode::PlacerMode {

    let typeid = TypeId::of::<Mesh>();
    //println!("PREPARING TO ADD STUFF TO PLACE MODE UI");
    //info!("PRIMARY WINDOW COUNT: {:#?}", primary_window.iter().len());
    for mut context in primary_window.iter_mut() {
        let ui_name = "prefab meshes";
        egui::SidePanel::left(ui_name).show(context.get_mut(), |ui| {
            ui.heading(ui_name);
            if let Some(folder) = folders.get(&model_folder.0) {
                let handles: Vec<Handle<Mesh>> = folder
                    .handles
                    .clone()
                    .into_iter()
                    .filter(|handle| handle.type_id() == typeid)
                    .map(|handle| handle.typed::<Mesh>())
                    .collect::<Vec<_>>();

                for mesh_handle in handles {
                    //let mesh = meshes.get(mesh_handle.clone()).expect("not loaded");
                    if let Some(path) = mesh_handle.path() {
                        let str_path = path.path().to_str().unwrap();

                        let model_name = str_path.split('/').last().unwrap_or_default().to_owned();
                        let spawn_button = ui.button(model_name.clone());

                        if spawn_button.clicked() {
                            //TODO! put raycasting code here
                            commands.spawn((
                                MaterialMeshBundle {
                                    mesh: mesh_handle.clone(),
                                    material: placer_materials.add(NeonGlowMaterial {
                                        color: Color::RED.into(),
                                    }),
                                    ..default()
                                },
                                Placer::from_path(str_path),
                                ColliderFlag::Convex,
                                Sensor,
                                Name::new(model_name.clone())
                            ));
                            *tool_mode = BuildToolMode::PlacerMode
                        }
                        //spawn display model for hovered over spawnables
                        if spawn_button.hovered() {
                            ui.label("show display model here!");
                            for (e, display_handle) in display_models.iter() {
                                if mesh_handle.path() != display_handle.path() {
                                    commands.entity(e).despawn()
                                }
                            }
                            if display_models.iter().len() < 1 {
                                display_model(&mut commands, &mut placer_materials, mesh_handle)
                            }
                        } else {
                            for (e, ..) in display_models.iter() {
                                commands.entity(e).despawn()
                            }
                        }
                    }
                }
            } else {
                ui.label("could not load folder...");
            }
        });
    }
    //}
}
