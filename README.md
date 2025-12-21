# Chartify Pro (Rust Version)

A high-performance data visualization and statistical analysis tool built with Rust and egui.

## Features

- 📊 Interactive Boxplot and Normal Quantile Plot
- 📈 Statistical analysis (Mean, Median, Std Dev, etc.)
- 🚀 High performance with async CSV loading
- 💾 Support for large datasets (100M+ rows)
- 🖥️ Cross-platform (macOS, Windows)

## Building

### Prerequisites

- [Rust](https://rustup.rs/) (stable)

### Build Release

```bash
cargo build --release
```

### Run

```bash
cargo run --release
```

## Creating macOS App Bundle

```bash
mkdir -p "Chartify Pro.app/Contents/MacOS"
cp target/release/chartify_pro "Chartify Pro.app/Contents/MacOS/Chartify Pro"
```

## GitHub Actions

Windows EXE is automatically built on push to `main` branch. Download from Actions artifacts.

## License

MIT
