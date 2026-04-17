# Installation

## From GitHub Releases (prebuilt binary)

Download the latest release for your platform from the [Releases page](https://github.com/Mahmoud-Emad/gruth/releases), then:

```bash
# Extract the archive (the download is a .tar.gz)
tar xzf gruth-macos-arm64.tar.gz    # or gruth-linux-amd64, gruth-linux-arm64

# Make it executable and move to your PATH
chmod +x gruth
sudo mv gruth /usr/local/bin/
```

Available binaries:

| Platform | File |
|----------|------|
| macOS Apple Silicon | `gruth-macos-arm64.tar.gz` |
| Linux x86_64 | `gruth-linux-amd64.tar.gz` |
| Linux ARM64 | `gruth-linux-arm64.tar.gz` |

## From source (requires Rust 1.70+)

```bash
cargo install --path .
```

Or clone and build:

```bash
git clone https://github.com/Mahmoud-Emad/gruth.git
cd gruth
cargo build --release
sudo cp target/release/gruth /usr/local/bin/
```

## Self-update

Once installed, gruth can update itself:

```bash
gruth update
```

The TUI also checks for updates on startup. If a newer version exists, you'll see `↑ X.Y.Z available` in the header bar.
