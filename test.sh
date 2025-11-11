#!/bin/bash

# Create a test video with moving elements to visualize the transposition effect
echo "Creating test video..."

cd "$(dirname "$0")"
mkdir -p res

# Create a simple test video: a white square moving from left to right
# This will clearly show the X-T transposition effect
ffmpeg -f lavfi -i color=c=black:s=640x480:d=20,format=yuv420p \
  -f lavfi -i color=c=white:s=100x100:d=20,format=yuva420p \
  -filter_complex "[1:v]setpts=PTS-STARTPTS[box];[0:v][box]overlay=x='50+t*20':y=190:shortest=1" \
  -r 30 -y res/test_input.mp4

echo "Test video created: res/test_input.mp4"
echo "  - 640×480 pixels"
echo "  - 5 seconds (150 frames at 30fps)"
echo "  - White square moving left to right"
echo ""
echo "Building the Rust program..."
cargo build --release

if [ $? -eq 0 ]; then
    echo ""
    echo "Running transposition..."
    ./target/release/video-transpose res/test_input.mp4 res/test_output.mp4

    echo ""
    echo "Done! Compare the videos:"
    echo "  Input:  res/test_input.mp4  (640×480, 150 frames)"
    echo "  Output: res/test_output.mp4 (150×480, 640 frames)"
    echo ""
    echo "In the output video:"
    echo "  - The moving square becomes a diagonal line"
    echo "  - Time now flows horizontally instead of through frames"
    echo "  - Each frame shows one vertical column from the original"
else
    echo "Build failed. Make sure FFmpeg libraries are installed."
fi
