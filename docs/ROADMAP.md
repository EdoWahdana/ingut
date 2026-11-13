# Roadmap Ingut

## M0 — Kerangka (sekarang)
- [x] Window GPUI dengan daftar contoh
- [x] Pastikan `cargo run` berhasil di macOS dengan `gpui-kit`

## M1 — MVP macOS
- [x] Watcher clipboard (polling ~400ms via `arboard`), abaikan duplikat berurutan
- [x] Simpan riwayat ke SQLite (`rusqlite`, bundled) di folder data aplikasi (`directories`)
- [x] Input pencarian + fuzzy search (`nucleo-matcher`)
- [x] Navigasi keyboard: ↑/↓ pilih, Enter salin, Esc tutup
- [x] Shortcut global Cmd+Shift+V untuk buka/tutup popup (`global-hotkey`)
- [x] Popup muncul di dekat kursor, tanpa titlebar, hilang saat kehilangan fokus
- [x] Batas riwayat (default 200 item)

## M2 — Nyaman dipakai
- [x] Pin item favorit (selalu di atas, sesuaikan dengan mode VIM)
- [x] Dukungan gambar
- [x] Tema terang/gelap mengikuti sistem

## M3 — Cross-platform
- [ ] Mac
- [ ] Windows
- [ ] Linux
- [x] GitHub Actions membangun biner macOS, Windows, dan Linux, lalu melampirkannya ke GitHub Release saat tag `v*`
