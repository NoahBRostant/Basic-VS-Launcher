mod pages;
use eframe::{egui, App, Frame};
use pages::{home::HomePage, versions::VersionPage, instances::InstancesPage, mods::ModsPage};
use pages::instances::InstanceCmd;
enum View { Home, Versions, Instances, Mods}
pub struct VsLauncherApp { view: View, home: HomePage, versions: VersionPage, instances: InstancesPage, selected_idx: Option<usize>, mods: ModsPage}
impl Default for VsLauncherApp {
    fn default() -> Self { Self { view: View::Home, home: HomePage::default(), versions: VersionPage::default(), instances: InstancesPage::default(), selected_idx: None, mods: ModsPage::default()} }
}
impl App for VsLauncherApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        eframe::egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Home").clicked()     { self.view = View::Home; }
                if ui.button("Versions").clicked() { self.view = View::Versions; }
                if ui.button("Instances").clicked() { self.view = View::Instances; }
                if ui.button("Mods").clicked() { self.view = View::Mods; }
            });
        });
        // run the current page and capture play-command if any
        let cmd = match self.view {
            View::Home => {
                self.home.ui(ctx);
                InstanceCmd::None
            }
            View::Versions => {
                self.versions.ui(ctx);
                InstanceCmd::None
            }
            View::Instances => self.instances.ui(ctx),     // returns InstanceCmd
            View::Mods => {
                self.mods.ui(ctx);
                InstanceCmd::None
            }
        };

        // handle the request after the borrow on self.instances is over
        if let InstanceCmd::Play(idx) = cmd {
            self.launch_instance(idx);
        }
        eframe::egui::TopBottomPanel::bottom("global_footer").show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_source("global_instance_select")
                    .selected_text(
                        self.selected_idx
                            .and_then(|i| self.instances.instances.get(i))
                            .map(|inst| inst.name.clone())
                            .unwrap_or_else(|| "(choose instance)".into()),
                    )
                    .show_ui(ui, |ui| {
                        for (idx, inst) in self.instances.instances.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_idx, Some(idx), &inst.name);
                        }
                    });

                let play_enabled = self.selected_idx.is_some();
                if ui.add_enabled(play_enabled, egui::Button::new("Play")).clicked() {
                    if let Some(idx) = self.selected_idx {
                        self.launch_instance(idx);
                    }
                }
            });
        });
    }
}

impl VsLauncherApp {
    fn launch_instance(&mut self, idx: usize) {
        use std::os::unix::fs::PermissionsExt;
        if let Some(inst) = self.instances.instances.get(idx) {
            let root = pages::versions::VersionPage::install_dir(&inst.version).join("vintagestory");
            let candidates = [
                root.join("Vintagestory"),
                root.join("run.sh"),
                root.join("Vintagestory.exe"), // future Windows port????????
            ];

            let bin = candidates.iter().find(|p| p.exists());
            let Some(bin) = bin else {
                self.instances.status_msg =
                    Some(format!("Executable not found for {}", inst.name));
                return;
            };

            // ensure executable bit
            if let Ok(meta) = std::fs::metadata(bin) {
                let mut perms = meta.permissions();
                if perms.mode() & 0o111 == 0 {
                    perms.set_mode(perms.mode() | 0o755);
                    let _ = std::fs::set_permissions(bin, perms);
                }
            }

            // run via bash -c '<path>'
            let result = std::process::Command::new("bash")
                .arg("-c")
                .arg(bin.to_string_lossy().to_string())
                .current_dir(root)
                .spawn();

            match result {
                Ok(_) => {
                    self.instances.status_msg =
                        Some(format!("Launched {}", inst.name));
                }
                Err(e) => {
                    self.instances.status_msg =
                        Some(format!("Launch error: {e}"));
                    eprintln!("launch failed: {e}");
                }
            }
        }
    }
}


fn main() -> eframe::Result<()> {
    eframe::run_native("Vintage Story Launcher", eframe::NativeOptions::default(), Box::new(|_| Box::<VsLauncherApp>::default()))
}
