# Chartify Pro

<div align="center">

**A high-performance data visualization and statistical analysis tool built with Rust**

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)

</div>

## ✨ Features

- **📊 Interactive Charts** - Boxplot and Normal Quantile Plot with zoom/drag support
- **📈 Statistical Analysis** - Mean, Median, Standard Deviation, Percentiles, P-values
- **🚀 High Performance** - Async CSV loading, handles 100M+ rows efficiently
- **� PPT Export** - Generate PowerPoint reports with 4 charts per slide
- **🎨 Visual Indicators** - Color-coded results for significant/non-significant differences
- **🖥️ Cross-Platform** - Native support for macOS and Windows

## 📸 Screenshots

| Feature | Description |
|---------|-------------|
| Multi-column layout | Charts auto-arrange based on window width |
| Interactive plots | Zoom and drag to explore data details |
| Statistics table | Complete statistical summary per group |

## 🚀 Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable, 1.70+)

### Build & Run

```bash
# Clone the repository
git clone https://github.com/MurphySunrise/Chartify_Pro_Rust.git
cd Chartify_Pro_Rust

# Build release version
cargo build --release

# Run the application
cargo run --release
```

## 📖 Usage

### Quick Start

1. **Load Data** - Click "📂 Browse" to select a CSV file
2. **Configure Analysis**
   - Select **Group Column** (categorical, e.g., "Treatment")
   - Select **Control Group** (baseline for comparison)
   - Select **Data Columns** (numeric columns to analyze)
3. **Calculate** - Click "▶ Start Calculation" to run analysis
4. **Explore Charts** - Interactive boxplot and QQ plot with zoom/drag
5. **Export Report** - Click "📄 Export PPT" to generate PowerPoint
6. **Open Report** - Click "📂 Open PPT" to view the generated file

### CSV Format Example

```csv
Group,DataType1,DataType2,DataType3
Control,1.2,3.4,5.6
Control,1.3,3.5,5.7
Test_A,1.5,3.8,6.0
Test_A,1.6,3.9,6.1
Test_B,1.1,3.2,5.4
Test_B,1.0,3.1,5.3
```

### Column Requirements

| Column Type | Description | Example |
|-------------|-------------|---------|
| **Group** | Categorical column for grouping | "Control", "Test_A", "Test_B" |
| **Data** | Numeric columns for statistical analysis | 1.2, 3.4, 5.6 |

### Chart Features

- **Boxplot**: Shows distribution with median, quartiles, and outliers
- **Normal Quantile Plot**: X-axis displays probability values (0.01, 0.05, 0.20, 0.25, 0.50, 0.75, 0.80, 0.95, 0.99)
  - Normal distribution appears as a straight line
- **Statistics Table**: N, Mean, Median, Std, P05, P95, (M-C)/σ, P-value

## 🏗️ Architecture

```
src/
├── main.rs          # Application entry point
├── gui/             # UI components (egui)
│   ├── app.rs       # Main application logic
│   ├── chart_viewer.rs  # Multi-column chart display
│   └── control_panel.rs # Settings panel
├── charts/          # Visualization
│   ├── plotter.rs   # Interactive egui_plot charts
│   └── renderer.rs  # PNG rendering for export
├── data/            # Data processing
│   └── processor.rs # CSV loading and processing
├── stats/           # Statistical calculations
│   └── calculator.rs # Mean, Std, P-values, etc.
└── ppt.rs           # PowerPoint generation
```

## 🔧 Technology Stack

- **[egui](https://github.com/emilk/egui)** - Immediate mode GUI
- **[egui_plot](https://docs.rs/egui_plot)** - Interactive plotting
- **[polars](https://pola.rs/)** - High-performance DataFrame
- **[plotters](https://plotters-rs.github.io/)** - PNG chart rendering
- **[zip](https://docs.rs/zip)** - PPTX generation

## 📦 Build Artifacts

### macOS App Bundle

```bash
mkdir -p "Chartify Pro.app/Contents/MacOS"
cp target/release/chartify_pro "Chartify Pro.app/Contents/MacOS/Chartify Pro"
```

### Windows

Windows EXE is automatically built via GitHub Actions on push to `main` branch.  
Download from [Actions](../../actions) → Latest workflow run → Artifacts.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

<div align="center">
Made with ❤️ and 🦀
</div>
