//! pages/instances.rs – create / list / delete instances
use std::{fs, path::PathBuf};

use dirs::data_local_dir;
use eframe::egui::{self, CentralPanel};
use serde::{Deserialize, Serialize};

use crate::pages::versions::VersionPage;

/*──────────────────── data ───────────────────*/
#[derive(Serialize, Deserialize, Clone)]
pub struct Instance {
    pub name:    String,
    pub version: String,
}

pub enum InstanceCmd {
    Play(usize),
    None,
}

pub struct InstancesPage {
    pub instances: Vec<Instance>,
    new_name:      String,
    new_version:   String,
    show_modal:    bool,
    pub status_msg: Option<String>,
    pending_delete: Option<usize>,
}

impl Default for InstancesPage {
    fn default() -> Self {
        Self {
            instances: Self::load_instances(),
            new_name: String::new(),
            new_version: String::new(),
            show_modal: false,
            status_msg: None,
            pending_delete: None,
        }
    }
}

/*────────────────── disk helpers ─────────────*/
impl InstancesPage {
    fn instances_file() -> PathBuf {
        data_local_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("vs_launcher/instances.json")
    }
    fn load_instances() -> Vec<Instance> {
        std::fs::read_to_string(Self::instances_file())
            .ok()
            .and_then(|txt| serde_json::from_str(&txt).ok())
            .unwrap_or_default()
    }
    fn save_instances(&self) {
        let path = Self::instances_file();
        if let Some(p) = path.parent() { let _ = fs::create_dir_all(p); }
        if let Ok(j) = serde_json::to_string_pretty(&self.instances) {
            let _ = std::fs::write(path, j);
        }
    }
    fn installed_versions() -> Vec<String> {
        let root = VersionPage::versions_dir();
        let mut v = Vec::new();
        if let Ok(rd) = fs::read_dir(root) {
            for e in rd.flatten() {
                if e.path().join("install").exists() {
                    if let Some(s) = e.file_name().to_str() { v.push(s.into()) }
                }
            }
        }
        v.sort();
        v
    }
    fn remove_instance(&mut self, idx: usize) {
        if let Some(inst) = self.instances.get(idx) {
            let folder = data_local_dir()
                .unwrap_or_else(|| PathBuf::from("~/.local/share"))
                .join("vs_launcher/instances")
                .join(&inst.name);
            if let Err(e) = fs::remove_dir_all(&folder) {
                self.status_msg = Some(format!("Delete error: {e}"));
                return;
            }
        }
        self.instances.remove(idx);
        self.save_instances();
        self.status_msg = Some("Instance deleted".into());
    }
}

/*──────────────────── UI ─────────────────────*/
impl InstancesPage {
    /// Draws the page and returns a play-request (if any)
    pub fn ui(&mut self, ctx: &egui::Context) -> InstanceCmd {
        let mut cmd = InstanceCmd::None;

        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Instances");
            if let Some(msg) = &self.status_msg { ui.label(msg); }

            /* list ------------------------------------------------ */
            self.pending_delete = None;

            for (idx, inst) in self.instances.iter().enumerate() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&inst.name).strong());
                            ui.label(format!("v{}", inst.version));
                        });
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui.button("🗑").clicked() {
                                    self.pending_delete = Some(idx);
                                }
                                if ui.button("▶").clicked() {
                                    cmd = InstanceCmd::Play(idx);
                                }
                            },
                        );
                    });
                });
                ui.add_space(6.0);
            }
            if let Some(i) = self.pending_delete.take() {
                self.remove_instance(i);
            }

            ui.separator();
            if ui.button("New instance…").clicked() {
                self.new_name.clear();
                self.new_version.clear();
                self.show_modal = true;
            }

            /* modal ---------------------------------------------- */
            if self.show_modal {
                egui::Window::new("Create instance")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.new_name);

                        ui.label("Game version:");
                        egui::ComboBox::from_id_source("ver_select")
                            .selected_text(if self.new_version.is_empty() {
                                "(choose)".into()
                            } else {
                                self.new_version.clone()
                            })
                            .show_ui(ui, |ui| {
                                for ver in Self::installed_versions() {
                                    ui.selectable_value(&mut self.new_version, ver.clone(), ver);
                                }
                            });

                        ui.horizontal(|ui| {
                            if ui.button("Create").clicked() {
                                if !self.new_name.is_empty() && !self.new_version.is_empty() {
                                    self.create_instance();
                                    self.show_modal = false;
                                }
                            }
                            if ui.button("Cancel").clicked() { self.show_modal = false; }
                        });
                    });
            }
        });

        cmd
    }

    fn create_instance(&mut self) {
        let root = data_local_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("vs_launcher/instances")
            .join(&self.new_name);
        let _ = fs::create_dir_all(root.join("mods"));

        self.instances.push(Instance {
            name: self.new_name.clone(),
            version: self.new_version.clone(),
        });
        self.save_instances();
    }
}
