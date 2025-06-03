//! src/pages/versions.rs – v0.4.1 with semver sorting & toggle

use std::{
    fs,
    io::{self, Read, Write},
    path::PathBuf,
    thread,
    time::Duration,
};

use compress_tools::{uncompress_archive, Ownership};
use crossbeam_channel::{unbounded, Receiver};
use dirs::data_local_dir;
use eframe::egui::{self, CentralPanel, ProgressBar};
use open;
use reqwest::blocking::Client;
use semver::Version;
use serde_json::Value;

/*────────── version record ─────────*/
#[derive(Clone)]
struct VersionInfo {
    ver:  String, // "1.20.11" or "1.21-rc.2"
    kind: String, // "stable" | "rc" | "preview" | "dev"
}

/*────────── background events ──────*/
enum ProgressEvent {
    Progress(f32), // 0.0‒1.0
    Error(String),
    Finished,
}

/*────────── task state ─────────────*/
enum TaskState {
    None,
    InProgress { ver: String, rx: Receiver<ProgressEvent> },
    Done,
}
impl Default for TaskState {
    fn default() -> Self {
        TaskState::None
    }
}

/*────────── UI state ───────────────*/
#[derive(Default)]
pub struct VersionPage {
    pub versions:      Vec<VersionInfo>,
    status_msg:    Option<String>,
    progress_frac: Option<f32>,
    task:          TaskState,

    /* ui controls */
    filter_text:    String,
    filter_channel: String,
    sort_ascending: bool,

    loaded_once: bool,
}

/*────────── UI driver ─────────────*/
impl VersionPage {
    pub fn ui(&mut self, ctx: &egui::Context) {
        self.poll_task(ctx);

        CentralPanel::default().show(ctx, |ui| {
            /* auto-load exactly once */
            if !self.loaded_once {
                self.fetch_versions();
                self.loaded_once = true;
            }
            if ui.button("Refresh").clicked() {
                self.fetch_versions();
            }

            /* ── filter + sort bar ────────────────────────── */
            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.text_edit_singleline(&mut self.filter_text);

                ui.label("Channel:");
                egui::ComboBox::from_id_source("chan_select")
                    .selected_text(if self.filter_channel.is_empty() {
                        "any"
                    } else {
                        &self.filter_channel
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.filter_channel, String::new(), "any");
                        ui.selectable_value(&mut self.filter_channel, "stable".into(), "stable");
                        ui.selectable_value(&mut self.filter_channel, "rc".into(), "rc");
                        ui.selectable_value(&mut self.filter_channel, "preview".into(), "preview");
                        ui.selectable_value(&mut self.filter_channel, "dev".into(), "dev");
                    });

                ui.separator();

                let sort_label = if self.sort_ascending { "Sort ▲" } else { "Sort ▼" };
                if ui.button(sort_label).clicked() {
                    self.sort_ascending = !self.sort_ascending;
                    self.sort_versions();
                }
            });

            /* status + progress */
            if let Some(msg) = &self.status_msg {
                ui.label(msg);
            }
            if let Some(p) = self.progress_frac {
                ui.add(ProgressBar::new(p).show_percentage());
            }

            ui.separator();

            /* version list */
            let mut to_download: Option<VersionInfo> = None;
            egui::ScrollArea::vertical().show(ui, |ui| {
                for v in self.versions.iter().filter(|v| self.matches_filter(v)) {
                    ui.horizontal(|ui| {
                        ui.label(format!("v{} ({})", v.ver, v.kind));

                        if self.is_installed(&v.ver) {
                            if ui.button("Open dir").clicked() {
                                let _ = open::that(Self::install_dir(&v.ver));
                            }
                        } else if ui.button("Download").clicked() {
                            to_download = Some(v.clone());
                        }
                    });
                }
            });

            if let Some(v) = to_download {
                self.spawn_download(v);
            }
        });

        self.maybe_schedule_ticker(ctx);
    }

    /*────────── filter helper ───────*/
    fn matches_filter(&self, v: &VersionInfo) -> bool {
        let text_ok = self.filter_text.is_empty() || v.ver.contains(&self.filter_text);
        let chan_ok = self.filter_channel.is_empty() || v.kind == self.filter_channel;
        text_ok && chan_ok
    }

    /*────────── semver sort ─────────*/
    fn sort_versions(&mut self) {
        self.versions.sort_by(|a, b| {
            let sa = Self::parse_semver(&a.ver);
            let sb = Self::parse_semver(&b.ver);
            let ord = sa.cmp(&sb); // None < Some(...)
            if self.sort_ascending {
                ord
            } else {
                ord.reverse()
            }
        });
    }

    fn parse_semver(raw: &str) -> Option<Version> {
        // Ensure "1.21" -> "1.21.0"
        let parts: Vec<&str> = raw.split('-').collect();
        let core = parts[0];
        let mut nums: Vec<&str> = core.split('.').collect();
        while nums.len() < 3 {
            nums.push("0");
        }
        let mut fix = nums.join(".");
        if parts.len() > 1 {
            fix.push('-');
            fix.push_str(parts[1]);
        }
        Version::parse(&fix).ok()
    }

    /*────────── paths / install check ───────*/
    pub(crate) fn versions_dir() -> PathBuf {
        data_local_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("vs_launcher/versions")
    }
    fn archive_path(ver: &str) -> PathBuf {
        Self::versions_dir().join(ver).join("vs_archive.tar.gz")
    }
    pub(crate) fn install_dir(ver: &str) -> PathBuf {
        Self::versions_dir().join(ver).join("install")
    }
    fn is_installed(&self, ver: &str) -> bool {
        let root = Self::install_dir(ver);
        root.join("vintagestory").exists()
            || root.join("vintagestory.exe").exists()
            || root.join("vintagestory/vintagestory").exists()
    }

    /*────────── fetch list from API ───────*/
    fn fetch_versions(&mut self) {
        self.status_msg = Some("Fetching list…".into());
        self.versions.clear();

        let url = "https://mods.vintagestory.at/api/gameversions";
        match Client::new().get(url).send().and_then(|r| r.json::<Value>()) {
            Ok(json) => {
                if let Some(arr) = json["gameversions"].as_array() {
                    for obj in arr {
                        let raw = obj["name"].as_str().unwrap_or("");
                        let name = raw.trim_start_matches('v');
                        let kind = obj["type"]
                            .as_str()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| {
                                if raw.contains("rc") {
                                    "rc".into()
                                } else if raw.contains("dev") {
                                    "dev".into()
                                } else if raw.contains("pre") {
                                    "preview".into()
                                } else {
                                    "stable".into()
                                }
                            });
                        self.versions.push(VersionInfo {
                            ver: name.to_string(),
                            kind,
                        });
                    }
                    self.sort_versions();
                    self.status_msg =
                        Some(format!("Found {} versions", self.versions.len()));
                } else {
                    self.status_msg = Some("Unexpected JSON shape".into());
                }
            }
            Err(e) => self.status_msg = Some(format!("Error: {e}")),
        }
    }

    /*────────── background thread mgmt ─────*/
    fn spawn_download(&mut self, v: VersionInfo) {
        if matches!(self.task, TaskState::InProgress { .. }) {
            self.status_msg = Some("A download is already running".into());
            return;
        }
        let (tx, rx) = unbounded();
        self.task = TaskState::InProgress {
            ver: v.ver.clone(),
            rx,
        };
        self.progress_frac = Some(0.0);
        self.status_msg = Some(format!("Downloading v{}…", v.ver));

        thread::spawn(move || {
            if let Err(e) = download_and_extract(&v, &tx) {
                let _ = tx.send(ProgressEvent::Error(e.to_string()));
            }
        });
    }

    fn poll_task(&mut self, ctx: &egui::Context) {
        let mut next_state: Option<TaskState> = None;

        if let TaskState::InProgress { ver, rx } = &mut self.task {
            let ver_name = ver.clone();
            let mut dirty = false;

            while let Ok(evt) = rx.try_recv() {
                match evt {
                    ProgressEvent::Progress(f) => {
                        self.progress_frac = Some(f);
                        dirty = true;
                    }
                    ProgressEvent::Finished => {
                        self.status_msg =
                            Some(format!("v{ver_name} downloaded & extracted"));
                        next_state = Some(TaskState::Done);
                        self.progress_frac = None;
                        dirty = true;
                    }
                    ProgressEvent::Error(e) => {
                        self.status_msg = Some(format!("Error: {e}"));
                        next_state = Some(TaskState::None);
                        self.progress_frac = None;
                        dirty = true;
                    }
                }
            }
            if dirty {
                ctx.request_repaint();
            }
        }
        if let Some(s) = next_state {
            self.task = s;
        }
    }

    fn maybe_schedule_ticker(&self, ctx: &egui::Context) {
        if matches!(self.task, TaskState::InProgress { .. }) {
            ctx.request_repaint_after(Duration::from_millis(10));
        }
    }
}

/*────────── worker thread ──────────*/
fn download_and_extract(
    v: &VersionInfo,
    tx: &crossbeam_channel::Sender<ProgressEvent>,
) -> io::Result<()> {
    let cdn_base = "https://cdn.vintagestory.at/gamefiles/stable/";
    let file = format!("vs_client_linux-x64_{}.tar.gz", v.ver);
    let url = format!("{cdn_base}{file}");

    let mut resp = Client::new()
        .get(&url)
        .send()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let total = resp.content_length().unwrap_or(0) as f32;

    let archive_path = VersionPage::archive_path(&v.ver);
    fs::create_dir_all(archive_path.parent().unwrap())?;
    let mut dst = fs::File::create(&archive_path)?;

    let mut downloaded = 0u64;
    let mut buf = [0u8; 8192];

    loop {
        let n = resp.read(&mut buf)?;
        if n == 0 {
            break;
        }
        dst.write_all(&buf[..n])?;
        downloaded += n as u64;
        if total > 0.0 {
            let _ = tx.send(ProgressEvent::Progress(downloaded as f32 / total));
        }
    }

    let install_dir = VersionPage::install_dir(&v.ver);
    fs::create_dir_all(&install_dir)?;
    let f = fs::File::open(&archive_path)?;
    uncompress_archive(&f, &install_dir, Ownership::Preserve)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let _ = tx.send(ProgressEvent::Finished);
    Ok(())
}
