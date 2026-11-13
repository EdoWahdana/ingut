# Ingut

> Dari kata **"ingat"**. Clipboard manager open source yang ringan, cantik, dan cross-platform (macOS, Linux, Windows), terinspirasi dari [Maccy](https://maccy.app) dan dibangun dengan [GPUI Kit](https://gpui-kit.com) di atas GPUI.

## Status
Kerangka awal (pre-alpha). Lihat [docs/ROADMAP.md](docs/ROADMAP.md).

## Menjalankan (macOS)
1. Pasang Xcode lengkap dari App Store, lalu `sudo xcode-select -s /Applications/Xcode.app`.
2. Pasang Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
3. `cargo run`

Build pertama agak lama karena GPUI dan komponennya ikut dikompilasi.

## Rilis
Push sebuah tag `v*`, misalnya `v0.1.0`. GitHub Actions membangun biner di macOS (Apple Silicon), Windows, dan Linux, lalu menempelkannya pada GitHub Release. Build di `main` dan pull request hanya menguji kompilasi, tanpa membuat rilis.

## Lisensi
MIT
