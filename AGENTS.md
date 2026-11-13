# Panduan untuk AI Agent (Cursor)

Baca file ini sebelum mengerjakan apa pun di repo ini. Rencana fitur ada di `docs/ROADMAP.md`. Daftar library, link dokumentasi, contoh kode, dan catatan per OS ada di `docs/REFERENCES.md`: baca itu sebelum menambah atau memakai dependency. Semua pekerjaan UI wajib mengikuti `docs/DESIGN.md`.

## Apa itu Ingut
Ingut (dari kata "ingat") adalah clipboard manager **open source, ringan, cantik, dan cross-platform** (macOS, Linux, Windows). Terinspirasi dari Maccy: popup kecil yang muncul lewat shortcut global, kolom pencarian di atas, daftar riwayat di bawah, pilih dengan keyboard lalu langsung tempel.

Target pengguna: orang biasa, bukan hanya developer. Jadi utamakan **sederhana dan langsung bisa dipakai** tanpa konfigurasi.

## Prinsip produk (urut prioritas)
1. **Ringan.** Idle CPU mendekati 0%, RAM kecil, startup instan. Jangan tambah dependency berat tanpa alasan kuat.
2. **Cepat dipakai dengan keyboard.** Semua aksi utama bisa tanpa mouse.
3. **Cantik dan tenang.** Tampilan minimal, sudut membulat, spasi lega, animasi halus seperlunya, ikut tema terang/gelap sistem.
4. **Privasi.** Semua data lokal. Tidak ada network, telemetry, atau akun. Hormati penanda "concealed/transient" dari password manager.
5. **Cross-platform.** Fitur inti harus jalan di ketiga OS; perbedaan OS disembunyikan di balik trait.

Bukan tujuan (jangan dibuat kecuali diminta): sinkronisasi cloud, akun, plugin system, AI features.

## Stack
- Rust (edition 2021), UI dengan **GPUI Kit** (`gpui-kit` dari crates.io). Ini satu-satunya dependency UI: sudah mem-pin versi GPUI yang cocok dan me-re-export GPUI, `gpui-base`, dan `gpui-component`. Import lewat `use gpui_kit::*;` dan `gpui_kit::component::...`. **Jangan** menambahkan `gpui` atau `gpui-component` secara terpisah, dan jangan memakai git dependency ke repo Zed.
- Pakai komponen GPUI Kit (Input, List/virtual list, Button, theme) sebelum membuat komponen sendiri. Dokumentasi: https://gpui-kit.com dan https://docs.rs/gpui-kit. GPUI masih pre-1.0, jadi cek API di versi yang terpasang, jangan menebak.
- `arboard` untuk clipboard, `global-hotkey` untuk shortcut, `tray-icon` untuk menu bar/tray, `rusqlite` (bundled) untuk riwayat, `nucleo-matcher` untuk fuzzy search, `enigo` untuk paste otomatis.

## Struktur
```
src/
  main.rs         # bootstrap GPUI dan window
  ui/             # semua view GPUI; tanpa akses langsung ke clipboard/DB
  clipboard/      # abstraksi clipboard lintas OS (trait ClipboardSource) + watcher
  storage/        # model ClipItem + penyimpanan (in-memory → SQLite)
docs/ROADMAP.md   # milestone dan checklist
docs/REFERENCES.md # library, link docs, contoh, inspirasi
docs/DESIGN.md    # design system: token, tipografi, motion, spesifikasi layar
```
Aturan lapisan: `ui` → memanggil service; `clipboard` dan `storage` tidak tahu soal UI. Kode khusus OS diberi `#[cfg(target_os = "...")]` dan hanya di `clipboard/` (atau modul platform baru).

## Cara kerja agent
- Kerjakan satu item roadmap per perubahan, kecil dan bisa di-review. Centang item di `docs/ROADMAP.md` setelah selesai.
- Setiap perubahan harus lolos `cargo build`, `cargo clippy -- -D warnings`, dan `cargo fmt --check`.
- Tambahkan unit test untuk logika non-UI (dedup, pencarian, batas riwayat, storage).
- Jangan blok UI thread: polling clipboard dan I/O database jalan di background (`cx.background_executor()`), hasilnya dikirim ke view.
- Hindari `unwrap()` di luar test dan bootstrap; pakai `anyhow::Result`.
- Komentar dan pesan commit boleh bahasa Indonesia atau Inggris, tapi konsisten per file.
- Saat menambah dependency, jelaskan alasannya di pesan commit dan pastikan mendukung ketiga OS.
