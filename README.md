# Basic-VS-Launcher

*A sleek, cross-platform launcher & instance manager for
[Vintage Story](https://www.vintagestory.at/).*
Written in **Rust** with **egui / eframe** for a lightweight native UI.

---

## ✨  Features

| Area | What it does |
|------|--------------|
| **Versions** | • Fetches every official release / RC / preview via Mod-DB API<br>• Semantic sorting & search filters<br>• 1-click download & progress bar<br>• Auto-detects already-installed clients |
| **Instances** | • Create, name, delete game profiles<br>• Per-instance mod folder<br>• Play / Delete buttons on each card |
| **Mods** | • Mod-DB browser<br>• Downloads / follows / comments counters |
| **UI** | • Super Basic UI with a dark theme |
| **Platform** | Linux (AppImage / .deb), macOS and Windows (build-untested) |

---

## 🚀  Quick start

```bash
# 1. Clone
git clone https://github.com/yourname/vs-launcher.git
cd vs-launcher

# 2. Build & run (debug)
cargo run

# or release build
cargo build --release
./target/release/vs_launcher
```

> **Note:** you need a Rust toolchain ≥ 1.72 and the GTK 3 dev headers
> (`sudo apt install libgtk-3-dev` on Debian/Ubuntu).

---

## 🛠  Packaging

| Output              | Command                                             |
| ------------------- | --------------------------------------------------- |
| **Portable tar.gz** | `cargo build --release && ./scripts/package-tar.sh` |
| **AppImage**        | `cargo appimage`                                    |
| **.deb**            | `cargo deb`                                         |

Scripts are in `scripts/`—CI generates all three on every tagged release.

---

## 📸  UI preview

| Home                       | Versions                       | Instances                       | Mods                       |
| -------------------------- | ------------------------------ | ------------------------------- | -------------------------- |
| ![](docs/screens/home.png) | ![](docs/screens/versions.png) | ![](docs/screens/instances.png) | ![](docs/screens/mods.png) |

*(Images captured on Linux, dark theme)*

---

## 🗺  Roadmap

* [ ] Better Mod-DB API Implementation
* [ ] Mod-DB search filters
* [ ] Mod-DB per instance install
* [ ] Auto-update launcher via GitHub releases
* [ ] Revised instance manager
* [ ] Drag-&-drop mod install
* [ ] Windows patcher + wineprefix helper
* [ ] Better UI

---

## 🤝  Contributing

1. Fork & branch (`feat/your-thing`)
2. `rustfmt` + `cargo clippy --all-targets -- -D warnings`
3. PR to `main` with a clear description / screenshots

All contributions must pass CI (build + fmt + tests).

---

## 📜  License

Licensed under **MIT** – see [LICENSE](LICENSE).

Vintage Story is © Anego Studios; this project is unaffiliated.
