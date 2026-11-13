# Referensi Ingut

Daftar library, contoh, dan inspirasi yang dipakai atau direncanakan. **Agent: sebelum memakai library di bawah, cek versi terbaru di crates.io dan baca docs.rs untuk versi itu. Jangan menebak API.**

## UI
| Nama | Fungsi di Ingut | Link | Catatan |
|---|---|---|---|
| GPUI Kit | Satu-satunya dependency UI: GPUI + gpui-base + gpui-component (75+ komponen) + ikon Lucide | [gpui-kit.com](https://gpui-kit.com) · [docs.rs](https://docs.rs/gpui-kit) · [longbridge/gpui-kit](https://github.com/longbridge/gpui-kit) | Sudah mem-pin versi GPUI yang cocok. Jangan tambah `gpui` terpisah. Ada skill untuk agent: `npx skills add longbridge/gpui-kit`. |
| GPUI | Fondasi render GPU di bawah GPUI Kit | [crates.io/gpui](https://crates.io/crates/gpui) · [contoh](https://github.com/zed-industries/zed/tree/main/crates/gpui/examples) | Dipakai lewat re-export `gpui_kit::*`, hanya sebagai bacaan. |
| Zed (source) | Contoh nyata pola GPUI: keybinding, actions, picker/fuzzy list | [zed-industries/zed](https://github.com/zed-industries/zed) | Baca di GitHub saja, jangan dijadikan dependency. `crates/picker` cocok untuk pola popup pencarian. |

## Clipboard dan sistem
| Nama | Fungsi di Ingut | Link | Catatan |
|---|---|---|---|
| arboard | Baca/tulis clipboard (teks & gambar) di 3 OS | [crates.io](https://crates.io/crates/arboard) · [docs](https://docs.rs/arboard) | Tidak punya event "clipboard berubah", jadi pakai polling ~400ms. Di Linux Wayland butuh fitur `wayland-data-control`. |
| global-hotkey | Shortcut global (Cmd+Shift+V) | [crates.io](https://crates.io/crates/global-hotkey) · [docs](https://docs.rs/global-hotkey) | macOS: harus dibuat di main thread. Wayland: tidak didukung, perlu jalur alternatif. |
| tray-icon | Ikon menu bar macOS / system tray Windows & Linux | [crates.io](https://crates.io/crates/tray-icon) · [docs](https://docs.rs/tray-icon) | Linux butuh `libappindicator`/`libayatana-appindicator` dan event loop gtk. |
| enigo | Simulasi tombol paste ke app aktif | [crates.io](https://crates.io/crates/enigo) · [docs](https://docs.rs/enigo) | macOS butuh izin Accessibility. |
| directories | Lokasi folder data aplikasi per OS | [crates.io](https://crates.io/crates/directories) | Simpan DB di `ProjectDirs::data_dir()`. |
| auto-launch | Jalankan saat login (M2) | [crates.io](https://crates.io/crates/auto-launch) | Opsional. |

## Data dan pencarian
| Nama | Fungsi di Ingut | Link | Catatan |
|---|---|---|---|
| rusqlite | Simpan riwayat di SQLite | [crates.io](https://crates.io/crates/rusqlite) · [docs](https://docs.rs/rusqlite) | Pakai fitur `bundled` agar tidak tergantung SQLite sistem. |
| nucleo-matcher | Fuzzy search cepat (dipakai editor Helix) | [crates.io](https://crates.io/crates/nucleo-matcher) · [repo](https://github.com/helix-editor/nucleo) | Alternatif lebih sederhana: `fuzzy-matcher`. |
| anyhow | Error handling | [crates.io](https://crates.io/crates/anyhow) | |

## Inspirasi produk
| Nama | Kenapa relevan | Link |
|---|---|---|
| Maccy | Inspirasi utama: UX popup, keyboard-first, abaikan item "concealed" | [maccy.app](https://maccy.app) · [p0deje/Maccy](https://github.com/p0deje/Maccy) |
| CopyQ | Clipboard manager cross-platform; contoh menangani Linux/Wayland | [hluk/CopyQ](https://github.com/hluk/CopyQ) |
| Raycast clipboard history | Referensi tampilan yang rapi dan modern | [raycast.com](https://www.raycast.com) |

## Penanda privasi clipboard
- macOS: abaikan item dengan tipe pasteboard `org.nspasteboard.ConcealedType` dan `org.nspasteboard.TransientType` ([nspasteboard.org](http://nspasteboard.org)).
- Windows: abaikan format `ExcludeClipboardContentFromMonitorProcessing`.
- Linux: abaikan MIME `x-kde-passwordManagerHint` bernilai `secret`.
