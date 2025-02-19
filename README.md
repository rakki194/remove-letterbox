# remove-letterbox

A command line tool to remove letterboxing from images. This tool uses the `imx` library for image processing and file system operations.

## Features

- Remove letterboxing from individual image files
- Process entire directories of images
- Recursive directory traversal option
- Adjustable threshold for letterbox detection
- Supports JPG, JPEG, PNG, WebP, and JXL formats
  - JXL files are automatically converted to PNG after processing
- Detailed logging of operations

## Installation

Clone the repository and build using Cargo:

```bash
cargo build --release
```

The binary will be available in `target/release/remove-letterbox`.

## Options

- `-i, --input <PATH>`: Input file or directory path (required)
- `-r, --recursive`: Process directories recursively
- `-t, --threshold <0-255>`: Threshold for letterbox detection (default: 10)
  - Higher values are more aggressive in detecting letterboxes
  - Lower values are more conservative
  - Recommended range: 5-30
- `-h, --help`: Print help
- `-V, --version`: Print version

## Usage

Remove letterboxing from a single image with default settings:

```bash
remove-letterbox -i movie_screenshot.jpg
```

Process a JXL image and convert to PNG:

```bash
remove-letterbox -i twilight_sparkle.jxl -t 15
# Result will be saved as twilight_sparkle.png
```

Process all images in the current directory with conservative threshold:

```bash
remove-letterbox -i . -t 5
```

Process all images in a directory and its subdirectories with custom threshold:

```bash
remove-letterbox -i ./photos -r -t 15
```

## How the Threshold Works

The threshold parameter (0-255) determines how dark a pixel needs to be to be considered part of the letterbox:

- Default value (10): Pixels with RGB values all below 10 are considered part of the letterbox
- Higher values (e.g., 20-30): More aggressive detection, may catch darker scene content
- Lower values (e.g., 5): More conservative, only removes very dark/black borders
- Choose based on your content:
  - Movies/Screenshots: 10-20 usually works well
  - Dark scenes: Try lower values (5-10)
  - Bright content: Can use higher values (15-30)

## JXL Support

The tool includes special handling for JPEG XL (JXL) files:

1. When a JXL file is processed, it is first converted to PNG format
2. The letterbox removal is performed on the PNG version
3. The original JXL file is removed, leaving only the processed PNG
4. All quality settings from the original JXL are preserved in the conversion

This allows you to process JXL files while maintaining image quality and taking advantage of PNG's lossless compression.

## License

This project is licensed under the MIT License.
