# LiDashBoard Packaging and Installation Guide

LiDashBoard (`lidash`) is distributed as precompiled, standalone executable binaries for Linux architectures. It operates purely in userspace as a single executable without requiring daemons, background services, container runtimes, or system modification.

## Release assets

Every official GitHub release publishes standalone binaries and a cryptographic checksum manifest:

| Asset Name | Target Architecture | Description |
| --- | --- | --- |
| `lidb-x86_64` | `x86_64` / `amd64` | Standalone Linux 64-bit executable |
| `lidb-aarch64` | `aarch64` / `arm64` | Standalone Linux ARM 64-bit executable |
| `SHA256SUMS.txt` | All architectures | Cryptographic SHA-256 verification hashes |

## Quick installation

Run the official POSIX-compliant installer script via `curl` or `wget`:

```bash
# Automated install (detects architecture, verifies checksum, installs to /usr/local/bin or ~/.local/bin)
curl -fsSL https://raw.githubusercontent.com/hami9/LiDB/main/scripts/install.sh | sh
```

### Installation options

```bash
# Install to custom directory
curl -fsSL https://raw.githubusercontent.com/hami9/LiDB/main/scripts/install.sh | sh -s -- --dir ~/.local/bin

# Install a specific release tag
curl -fsSL https://raw.githubusercontent.com/hami9/LiDB/main/scripts/install.sh | sh -s -- --tag v0.0.6

# Preview installation actions without writing files
curl -fsSL https://raw.githubusercontent.com/hami9/LiDB/main/scripts/install.sh | sh -s -- --dry-run
```

## Manual installation & verification

To verify and install release binaries manually:

1. Download the release binary for your architecture and the checksum manifest:
   ```bash
   TAG="v0.0.6"
   ARCH="x86_64" # or "aarch64"
   curl -fsSL -O "https://github.com/hami9/LiDB/releases/download/${TAG}/lidb-${ARCH}"
   curl -fsSL -O "https://github.com/hami9/LiDB/releases/download/${TAG}/SHA256SUMS.txt"
   ```

2. Verify the SHA-256 checksum:
   ```bash
   grep "lidb-${ARCH}" SHA256SUMS.txt | sha256sum -c -
   ```

3. Make the binary executable and move it to your system PATH:
   ```bash
   chmod +x "lidb-${ARCH}"
   sudo mv "lidb-${ARCH}" /usr/local/bin/lidash
   ```

4. Verify installation:
   ```bash
   lidash --version
   lidash doctor
   ```

## Upgrading

To upgrade an existing installation to the latest release, rerun the installer script:

```bash
curl -fsSL https://raw.githubusercontent.com/hami9/LiDB/main/scripts/install.sh | sh
```

The installer verifies the new binary's checksum and atomically replaces the previous binary.

## Uninstallation

To remove `lidash` completely from your system:

```bash
# Using the installer script:
curl -fsSL https://raw.githubusercontent.com/hami9/LiDB/main/scripts/install.sh | sh -s -- --uninstall

# Or manually:
rm -f /usr/local/bin/lidash
rm -f ~/.local/bin/lidash
```

LiDB creates zero daemon configurations, systemd units, background services, configuration databases, or hidden user files. Deleting the single `lidash` executable fully removes the application from the system.

## System requirements and resource limits

- **Operating System:** Linux (kernel >= 2.6 for basic procfs metrics; kernel >= 4.16 with BTF for optional eBPF lifecycle instrumentation).
- **Architecture:** `x86_64` (Intel/AMD) or `aarch64` (ARM64).
- **Privileges:** Default unprivileged non-root user. Optional eBPF requires `CAP_BPF` or root.
- **Memory Footprint:** Typically < 15 MB RSS during live monitoring.
- **Disk Footprint:** Single executable (~5 MB), 0 persistent disk writes during normal operation.
- **Terminal Requirements:** Any standard terminal supporting VT100 / ANSI escape sequences (minimum dimensions 35x10).
