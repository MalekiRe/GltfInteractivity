use std::collections::HashMap;

use eframe::{App, CreationContext, Storage};
use eframe::emath::Pos2;
use egui::{Color32, Style, Ui};
use egui_snarl::{
    ui::{AnyPins, PinInfo, SnarlStyle, SnarlViewer},
    InPin, InPinId, NodeId, OutPin, OutPinId, Snarl,
};
use serde::{Deserialize, Serialize};
use interactivity::{ExistingValues, get_registry, MathAdd, MathPi, NodeArchetype, NodeArchetypeIncomplete, NodeArchetypes, NodeBehavior, NodeBehaviors, OutputFlowSocket, OutputValueSocket, PrintNode, RegisterNode, SequenceNode};

const STRING_COLOR: Color32 = Color32::from_rgb(0x00, 0xb0, 0x00);
const NUMBER_COLOR: Color32 = Color32::from_rgb(0xb0, 0x00, 0x00);
const IMAGE_COLOR: Color32 = Color32::from_rgb(0xb0, 0x00, 0xb0);
const UNTYPED_COLOR: Color32 = Color32::from_rgb(0xb0, 0xb0, 0xb0);
pub fn main() {
    register();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    eframe::run_native(
        "egui-snarl demo",
        native_options,
        Box::new(|cx| Box::new(DemoApp::new(cx))),
    ).unwrap();
}

pub fn register() {
    MathAdd::register();
    MathPi::register();
    SequenceNode::register();
    PrintNode::register();
}

pub struct DemoApp {
    snarl: Snarl<NodeArchetypeIncomplete>,
    style: SnarlStyle,
    interactivity_viewer: InteractivityViewer,
}

impl DemoApp {
    pub fn new(cx: &CreationContext) -> Self {
        let snarl = match cx.storage {
            None => Snarl::new(),
            Some(storage) => {
                let snarl = storage
                    .get_string("snarl")
                    .and_then(|snarl| serde_json::from_str(&snarl).ok())
                    .unwrap_or_else(Snarl::new);

                snarl
            }
        };

        let style = match cx.storage {
            None => SnarlStyle::new(),
            Some(storage) => {
                let style = storage
                    .get_string("style")
                    .and_then(|style| serde_json::from_str(&style).ok())
                    .unwrap_or_else(SnarlStyle::new);

                style
            }
        };

        let interactivity_viewer = match cx.storage {
            None => InteractivityViewer { number_nodes: 0 },
            Some(storage) => {
                let interactivity_viewer = storage
                    .get_string("interactivity_viewer")
                    .and_then(|interactivity_viewer| serde_json::from_str(&interactivity_viewer).ok())
                    .unwrap_or_else(|| InteractivityViewer { number_nodes: 0 });
                interactivity_viewer
            }
        };

        DemoApp { snarl, style, interactivity_viewer }
    }
}

impl App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_dark_light_mode_switch(ui);

                if ui.button("Clear All").clicked() {
                    self.snarl = Default::default();
                }

                if ui.button("run").clicked() {
                    let mut node_archetypes = NodeArchetypes(HashMap::default());
                    let mut behaviors = NodeBehaviors(HashMap::default());
                    let mut sequence = 0;
                    for node_archetype in self.snarl.nodes() {
                        let registry = get_registry();
                        let registry = registry.lock().unwrap();
                        let (_, node_func) = registry.get(&node_archetype.name).unwrap();
                        let node = node_func(node_archetype.node_id);
                        behaviors.0.insert(node_archetype.node_id, node);
                        if node_archetype.name.as_str() == "flow/sequence" {
                            sequence = node_archetype.node_id;
                        }
                        node_archetypes.0.insert(
                            node_archetype.node_id,
                            node_archetype.clone().into()
                        );
                    }
                    let mut existing_values = ExistingValues::default();
                    existing_values.flow(sequence, &node_archetypes, &mut behaviors);
                }

            });

        });

        egui::SidePanel::left("style").show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui_probe::Probe::new("Snarl style", &mut self.style).show(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
             self.snarl
                 .show(&mut self.interactivity_viewer, &self.style, egui::Id::new("snarl"), ui);
        });

    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let snarl = serde_json::to_string(&self.snarl).unwrap();
        storage.set_string("snarl", snarl);

        let style = serde_json::to_string(&self.style).unwrap();
        storage.set_string("style", style);

        let interactivity_viewer = serde_json::to_string(&self.interactivity_viewer).unwrap();
        storage.set_string("interactivity_viewer", interactivity_viewer);
    }
}

#[derive(Serialize, Deserialize)]
struct InteractivityViewer {
    number_nodes: usize
}

impl SnarlViewer<NodeArchetypeIncomplete> for InteractivityViewer {
    fn title(&mut self, node: &NodeArchetypeIncomplete) -> String {
        node.name.clone()
    }

    fn outputs(&mut self, node: &NodeArchetypeIncomplete) -> usize {
        node.output_value_sockets.len() + node.output_flow_sockets.len()
    }

    fn inputs(&mut self, node: &NodeArchetypeIncomplete) -> usize {
        node.input_value_sockets.len() + node.input_flow_sockets.len()
    }

    fn show_input(&mut self, pin: &InPin, ui: &mut Ui, scale: f32, snarl: &mut Snarl<NodeArchetypeIncomplete>) -> PinInfo {
        let node = &snarl[pin.id.node];
        let color_type = if pin.remotes.len() == 0 { UNTYPED_COLOR } else { NUMBER_COLOR };
        if node.input_value_sockets.len() <= pin.id.input {
            PinInfo::square().with_fill(color_type)
        } else {
            PinInfo::circle().with_fill(color_type)
        }
    }

    fn show_output(&mut self, pin: &OutPin, ui: &mut Ui, scale: f32, snarl: &mut Snarl<NodeArchetypeIncomplete>) -> PinInfo {
        let node = &snarl[pin.id.node];
        let color_type = if pin.remotes.len() == 0 { UNTYPED_COLOR } else { NUMBER_COLOR };
        if node.output_value_sockets.len() <= pin.id.output {
            PinInfo::square().with_fill(color_type)
        } else {
            PinInfo::circle().with_fill(color_type)
        }
    }

    fn input_color(&mut self, pin: &InPin, style: &Style, snarl: &mut Snarl<NodeArchetypeIncomplete>) -> Color32 {
        let node = &snarl[pin.id.node];
        let color_type = if pin.remotes.len() == 0 { UNTYPED_COLOR } else { NUMBER_COLOR };
        color_type
    }

    fn output_color(&mut self, pin: &OutPin, style: &Style, snarl: &mut Snarl<NodeArchetypeIncomplete>) -> Color32 {
        let node = &snarl[pin.id.node];
        let color_type = if pin.remotes.len() == 0 { UNTYPED_COLOR } else { NUMBER_COLOR };
        color_type
    }

    fn graph_menu(&mut self, pos: Pos2, ui: &mut Ui, scale: f32, snarl: &mut Snarl<NodeArchetypeIncomplete>) {
        ui.label("Add node");
        let registry = get_registry();
        for (key, (value, _)) in registry.lock().unwrap().iter() {
            if ui.button(key).clicked() {
                snarl.insert_node(pos, value(self.number_nodes as u32));
                self.number_nodes += 1;
            }
        }
    }

    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<NodeArchetypeIncomplete>) {
        snarl.connect(from.id, to.id);
        let output_value_socket = snarl.get_node(from.id.node).unwrap();

        if from.id.output < output_value_socket.output_value_sockets.len() {
            let output_value_socket = output_value_socket.output_value_sockets.get(from.id.output).unwrap().clone();
            let input_value_socket = snarl.get_node_mut(to.id.node).unwrap();
            let input_value_socket = input_value_socket.input_value_sockets.get_mut(to.id.input).unwrap();
            input_value_socket.output_value_socket.replace(output_value_socket.clone());
        } else {
            let output_flow_socket = output_value_socket.output_flow_sockets.get(from.id.output - output_value_socket.output_value_sockets.len()).unwrap().clone();
            let input_flow_socket = snarl.get_node_mut(to.id.node).unwrap();
            let input_flow_socket = input_flow_socket.input_flow_sockets.get_mut(to.id.input - input_flow_socket
                .input_value_sockets.len()).unwrap();
            input_flow_socket.output_flow_socket.replace(output_flow_socket);
            let input_flow_socket = input_flow_socket.clone();
            let output_value_socket = snarl.get_node_mut(from.id.node).unwrap();
            let output_flow_socket = output_value_socket.output_flow_sockets.get_mut(from.id.output - output_value_socket.output_value_sockets.len()).unwrap();
            output_flow_socket.input_flow_socket.replace(Box::new(input_flow_socket));
        }
    }
}