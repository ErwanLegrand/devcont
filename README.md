# Devcont

Run [devcontainers](https://containers.dev/) from CLI or CI.

## Installation

`cargo install --path .`

## Usage

Run from a project that contains `.devcontainer/devcontainer.json` (or `.devcontainer.json`):

```sh
devcont                  # start (or resume) the container
devcont rebuild          # destroy and rebuild the container
devcont rebuild --no-cache  # rebuild without layer cache
```

Both commands accept an optional `[dir]` argument to target a different directory.

### Inspecting Containers (side-effect free)

These commands read `devcontainer.json` without starting the container or running
any hooks (`initializeCommand` etc.).

```sh
# Print the deterministic container name (one line, no trailing space)
devcont container-name
devcont container-name /path/to/project

# Print a JSON document with container metadata
devcont info
devcont info /path/to/project

# Skip the engine probe (omits exists/running; never invokes docker/podman)
devcont info --no-probe
```

Example `info` output:

```json
{
  "container": "devcont-myproject",
  "image": "ubuntu:24.04",
  "workspace": "/workspace",
  "config_dir": "/path/to/.devcontainer",
  "engine": "docker",
  "exists": false,
  "running": false
}
```

Exit codes for `container-name`:

| Code | Meaning |
|------|---------|
| `0`  | Name printed successfully |
| `2`  | `devcontainer.json` not found or parse error |
| `3`  | Config loaded but name could not be derived |

Exit codes for `info`:

| Code | Meaning |
|------|---------|
| `0`  | Info printed successfully |
| `2`  | `devcontainer.json` not found or parse error |
| non-zero | Engine probe failed (not applicable with `--no-probe`) |

## SSH Agent

`devcont` forwards your SSH agent socket into the container via `$SSH_AUTH_SOCK`
automatically — no key copying needed.

## Configuration

`~/.config/devcont/config.toml`:

```toml
# "docker" (default), "podman", "nerdctl", or "apple" (macOS only)
provider = "docker"

# Dotfiles to copy into the container (paths relative to $HOME)
dotfiles = [".zshrc", ".config/nvim"]
```

`~/.gitconfig` is always copied when present.

## Supported Engines

docker, docker-compose, podman, podman-compose, nerdctl, apple (macOS only)

### Podman Container Runtime

Podman is a daemonless, open source, Linux-native tool designed to make it easy to find, run, build, and share containerized applications. Devcont provides enhanced Podman support with rootless mode detection, configurable user namespaces, and optimized performance.

**Basic Usage:**

```toml
# ~/.config/devcont/config.toml
provider = "podman"
```

**Advanced Configuration:**

```toml
# Example: Configure Podman for rootless mode with custom settings
[podman]
userns_mode = "keep-id"      # User namespace mode: "keep-id", "host", or "auto"
disable_selinux = true      # Disable SELinux labeling for better compatibility
```

**Features:**

1. **Rootless Support** - Automatic detection and optimization for rootless Podman installations
2. **User Namespace Configuration** - Configurable user namespace modes for different security requirements
3. **SELinux Control** - Toggle SELinux labeling based on your security needs
4. **Availability Detection** - Automatic verification that Podman is installed and running
5. **Performance Optimization** - Optimized command execution with proper environment setup
6. **Full devcontainer.json Support** - Complete compatibility with the Dev Containers specification

**Configuration Options:**

| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| `userns_mode` | `"keep-id"`, `"host"`, `"auto"` | `"keep-id"` | User namespace mode for container isolation |
| `disable_selinux` | `true`, `false` | `true` | Disable SELinux labeling for compatibility |

**Best Practices:**

### Rootless Podman

For best results with rootless Podman:

```bash
# Install Podman in rootless mode
curl -fsSL https://get.rpmfusion.org | bash
sudo dnf install -y podman

# Set up rootless storage
podman system migrate

# Configure linger for the user
sudo loginctl enable-linger $(whoami)
```

### Security Configuration

For enhanced security in rootful environments:

```toml
# ~/.config/devcont/config.toml
[podman]
userns_mode = "keep-id"
disable_selinux = false  # Keep SELinux enabled for rootful
```

### Troubleshooting

**Common Issues and Solutions:**

1. **Permission denied errors**
   - Ensure proper storage setup: `podman system migrate`
   - Check user namespace configuration
   - Verify `/etc/subuid` and `/etc/subgid` are properly configured

2. **Podman not detected**
   - Verify Podman is installed: `podman --version`
   - Check Podman service is running: `podman info`
   - Ensure Podman is in your PATH

3. **SELinux conflicts**
   - Try disabling SELinux: `disable_selinux = true`
   - Check SELinux context: `ls -Z`
   - Review audit logs: `ausearch -m AVC -ts recent`

4. **Rootless network issues**
   - Configure slirp4netns: `podman machine init`
   - Check firewall settings
   - Verify network namespace configuration

**Limitations:**

- **Podman Machine (macOS/Windows):** Limited support for Podman Machine VMs
- **User Namespaces:** Requires proper system configuration (`/etc/subuid`, `/etc/subgid`)
- **Storage Drivers:** Performance varies by storage driver configuration
- **Networking:** Rootless networking has some limitations compared to rootful

### Nerdctl (containerd)

Nerdctl is a Docker-compatible CLI for containerd. It provides a familiar Docker-like experience while using containerd as the container runtime, making it popular in Kubernetes environments.

```toml
# ~/.config/devcont/config.toml
provider = "nerdctl"
```

**Requirements:**
- `nerdctl` CLI installed
- `containerd` service running

**Features:**
- Docker-compatible CLI commands
- Full devcontainer.json specification support
- Automatic SSH agent forwarding
- Build support via BuildKit

### Apple Container Runtime

Apple's container runtime is supported on macOS 14.0+ (Sonoma) and provides native Linux container support optimized for Apple Silicon. To use Apple container:

```toml
# ~/.config/devcont/config.toml
provider = "apple"
```

**Requirements:**
- macOS 14.0+ (Sonoma or later)
- Apple Silicon (M1/M2) recommended
- Local Network access enabled in Security & Privacy settings
- `container` CLI installed (part of macOS)

**Features:**
- Native Linux container support on macOS
- Optimized for Apple Silicon performance
- Integrated with macOS virtualization framework
- Automatic SSH agent forwarding
- Full devcontainer.json specification support

**Limitations:**
- macOS only (not available on Linux/Windows)
- Requires macOS Sonoma or later
- Some Linux distributions may have compatibility limitations

## Supported `devcontainer.json` Fields

| Field | Notes |
|---|---|
| `name` | required |
| `image` | pull a pre-built image |
| `build.dockerfile`, `build.args` | build from a Dockerfile |
| `forwardPorts` | host↔container port mapping |
| `remoteEnv` | environment variables injected at runtime |
| `remoteUser` | user inside the container (default: `root`) |
| `workspaceFolder` | working directory (default: `/workspace`) |
| `dockerComposeFile` + `service` | compose mode |
| `runArgs` | extra `docker/podman create` arguments |
| `overrideCommand` | keep container alive with a sleep loop |
| `shutdownAction` | `none` / `stopContainer` / `stopCompose` |
| `mounts` | additional bind/volume mounts |
| `initializeCommand` | host — before container creation |
| `onCreateCommand` | container — after first create |
| `updateContentCommand` | container — after content update |
| `postCreateCommand` | container — after first create (post-setup) |
| `postStartCommand` | container — after each start |
| `postAttachCommand` | container — after each attach |

Hooks accept a string (`sh -c` form) or an array (no shell interpolation).
