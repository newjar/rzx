# RZX Compressor

A modern, fast compression tool written in Rust with native support for the `.rzx` archive format.

## Features

- 🚀 **Fast compression** - Multi-threaded compression and decompression
- 🔧 **Multiple algorithms** - DEFLATE, LZMA, and Zstandard support
- 🎯 **Smart algorithm selection** - Automatically chooses best algorithm per file type
- 🛡️ **Data integrity** - Built-in checksum verification
- 🌍 **Cross-platform** - Works on Windows, Linux, and macOS
- 📊 **Progress indicators** - Real-time progress bars for large operations
- 🎨 **Modern CLI** - Colored output and intuitive commands

## Installation

### From Source

```bash
git clone https://github.com/yourusername/rzx-compressor.git
cd rzx-compressor
cargo build --release
```

The binary will be available at `target/release/rzx`

### Using Cargo

```bash
cargo install rzx-compressor
```

## Usage

### Create Archive

```bash
# Basic compression
rzx create -o archive.rzx file1.txt file2.jpg folder/

# With specific algorithm and level
rzx create -o archive.rzx -a lzma -l 9 documents/

# Exclude patterns
rzx create -o backup.rzx -x "*.tmp" -x "*.log" project/
```

### Extract Archive

```bash
# Extract to current directory
rzx extract archive.rzx

# Extract to specific directory
rzx extract archive.rzx -o /path/to/extract/

# Force overwrite existing files
rzx extract archive.rzx -f
```

### List Contents

```bash
# Simple listing
rzx list archive.rzx

# Detailed listing
rzx list archive.rzx -d
```

### Test Archive

```bash
# Test archive integrity
rzx test archive.rzx

# Verbose testing
rzx test archive.rzx -v
```

### Show Archive Info

```bash
# Display archive information
rzx info archive.rzx
```

## RZX Format

The `.rzx` format is designed for:

- **Performance** - Optimized for modern multi-core processors
- **Flexibility** - Multiple compression algorithms in single archive
- **Extensibility** - Forward-compatible format design
- **Reliability** - Strong error detection and recovery

### Format Specification

```
RZX Archive Structure:
┌─────────────────┐
│ RZX Header      │ (32 bytes)
├─────────────────┤
│ File Metadata   │ (variable)
├─────────────────┤
│ Compressed Data │ (variable)
└─────────────────┘
```

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Benchmarking

```bash
cargo bench
```

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under of MIT license ([LICENSE-MIT](LICENSE-MIT))

## Roadmap

- [x] Basic CLI framework
- [ ] RZX format implementation
- [ ] Compression algorithms integration
- [ ] Multi-threading support
- [ ] Cross-platform testing
- [ ] GUI version
- [ ] Browser extension support

## Acknowledgments

- Rust compression ecosystem
- 7-Zip for inspiration
- Community feedback and contributions
