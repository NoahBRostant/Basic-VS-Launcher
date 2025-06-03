use std::sync::mpsc::{channel, Receiver};

use eframe::egui::{self, CentralPanel, ScrollArea};
use reqwest::blocking::Client;
use serde::Deserialize;

/*──────── data model ────────*/
#[derive(Deserialize, Debug)]
struct ApiMod {
    #[serde(alias = "modid", alias = "id")]
    id: u32,
    #[serde(alias = "displayname", alias = "modname", alias = "name", default)]
    displayname: String,
    #[serde(alias = "authorname", alias = "author", default)]
    authorname: String,
    #[serde(default)]
    downloadcount: u32,
    #[serde(alias = "followercount", default)]
    followcount: u32,
    #[serde(default)]
    commentcount: u32,
}

/*──────── page state ────────*/
pub struct ModsPage {
    mods: Vec<ApiMod>,
    next_page: usize,
    total_pages: usize,
    loading: bool,
    rx: Option<Receiver<Result<(Vec<ApiMod>, usize), String>>>,
}

impl Default for ModsPage {
    fn default() -> Self {
        Self {
            mods: Vec::new(),
            next_page: 1,
            total_pages: 0,
            loading: false,
            rx: None,
        }
    }
}

/*──────── worker fetch ───────*/
fn fetch_page(page: usize, size: usize) -> Result<(Vec<ApiMod>, usize), String> {
    let url = format!(
        "https://mods.vintagestory.at/api/mods?page={page}&pageSize={size}&sort=latest"
    );
    let json: serde_json::Value = Client::new()
        .get(url)
        .send()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;

    let total_pages = json["totalPages"]
        .as_u64()
        .or_else(|| json["totalpages"].as_u64())
        .unwrap_or(1) as usize;

    let mut mods: Vec<ApiMod> =
        serde_json::from_value(json["mods"].clone()).map_err(|e| e.to_string())?;
    mods.truncate(size); // safety cap
    Ok((mods, total_pages))
}

/*──────── egui UI ───────────*/
impl ModsPage {
    pub fn ui(&mut self, ctx: &egui::Context) {
        /* first run — load 50 */
        if self.mods.is_empty() && !self.loading {
            self.start_fetch(1, 96);
        }

        /* poll worker */
        if let Some(rx) = &self.rx {
            if let Ok(result) = rx.try_recv() {
                self.loading = false;
                self.rx = None;
                if let Ok((mut mods, total)) = result {
                    self.total_pages = total;
                    self.mods.append(&mut mods);
                    // next_page already bumped in start_fetch
                }
            }
        }

        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Loaded {}", self.mods.len()));
                if self.loading {
                    ui.spinner();
                }
            });
            ui.separator();

            ScrollArea::both().show(ui, |ui| {
                egui::Grid::new("mods_grid")
                    .num_columns(4)
                    .spacing([16.0, 16.0])
                    .show(ui, |ui| {
                        let mut need_more = false;

                        for (i, m) in self.mods.iter().enumerate() {
                            /* ----- render cell ----- */
                            ui.vertical(|ui| {
                                let title = if m.displayname.is_empty() {
                                    format!("ID {}", m.id)
                                } else {
                                    m.displayname.clone()
                                };
                                ui.label(egui::RichText::new(title).strong());
                                if !m.authorname.is_empty() {
                                    ui.label(egui::RichText::new(&m.authorname).small());
                                }
                                ui.label(
                                    egui::RichText::new(format!(
                                        "⬇ {}  👥 {}  💬 {}",
                                        m.downloadcount, m.followcount, m.commentcount
                                    ))
                                    .small(),
                                );
                            });

                            if (i + 1) % 4 == 0 {
                                ui.end_row();
                            }

                            /* mark if we've reached 80 % of current list */
                            if !self.loading
                                && self.next_page <= self.total_pages
                                && i >= self.mods.len() * 4 / 5  // 80 %
                            {
                                need_more = true;
                            }
                        }

                        /* after grid draw = safe mut-borrow */
                        if need_more {
                            self.start_fetch(self.next_page, 24);
                        }
                    });
            });
        });
    }

    fn start_fetch(&mut self, page: usize, size: usize) {
        self.loading = true;
        let (tx, rx) = channel();
        self.rx = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(fetch_page(page, size));
        });
        self.next_page = page + 1; // set up for next time
    }
}
