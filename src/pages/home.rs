use eframe::egui::{self, CentralPanel};
#[derive(Default)]
pub struct HomePage;
impl HomePage {
    pub fn ui(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Vintage Story Launcher v0.5.0");
            ui.label("by nbrostant");
        });
    }
}
