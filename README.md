## 🎬 Video Transpose: Swapping Space & Time in Videos

The program **swaps the horizontal (X) axis with time (T)**, creating a fascinating transformation:

**Normal video:**

- Each frame is X×Y pixels
- T frames total
- Time flows *between* frames

**Transposed video:**

- Each frame is T×Y pixels (time becomes width!)
- X frames total (width becomes frame count!)
- Time flows *horizontally within* each frame

### Visual Example

If you have a ball moving left-to-right across the screen:

- **Input**: Ball appears in different positions across sequential frames
- **Output**: Ball appears as a diagonal streak, with its entire motion visible in the spatial layout

## 📦 What's Included

### Core Files

- **`src/main.rs`** - Efficient Rust implementation with progress bars
- **`Cargo.toml`** - Project configuration
- **`test.sh`** - Automated test with moving square demo

## 🚀 Quick Start

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Install FFmpeg libraries (Ubuntu/Debian)
sudo apt-get install ffmpeg libavcodec-dev libavformat-dev \
  libavutil-dev libswscale-dev pkg-config clang
  
# For mac OS, use Homebrew:
# brew install ffmpeg pkg-config
# Or for M chips:
# arch -arm64 brew install ffmpeg pkg-config

# 3. Build
cd video_transpose
cargo build --release

# 4. Run test
./test.sh

# 5. Use with your video
./target/release/video_transpose input.mp4 output.mp4
```

## ⚡ Key Features

- **Efficient**: Written in Rust for speed & safety
- **Complete**: Full FFmpeg integration, any format
- **Visual**: Progress bars during processing
- **Well-Documented**: 6 comprehensive guides

## 🎨 Creative Applications

1. Slit-scan photography effects
2. Motion visualization and analysis
3. Experimental video art
4. Temporal pattern discovery
5. Scientific motion studies

The effect creates surreal, artistic transformations where static objects become motion trails and moving objects create
complex geometric patterns. Perfect for creative projects!
