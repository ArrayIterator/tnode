# TRAY NODE

**High-Performance Unified Management System.**
## REQUIREMENTS

- Linux Operating System (Ubuntu 20.04 or higher recommended)
- CPU with AVX2 support (for optimal performance, but should work on older CPUs)
- Minimum 1GB For Running Application 8GB or more recommended for larger workloads
- Disks with good I/O performance (SSD recommended - NVMe preferred for best performance), Minimum 10GB of free disk space for application and data storage (depending on workload)
  it will highly recommended to use NVMe SSD for best performance, especially for workloads with high I/O demands. For development it will need 25GB of free disk space to build the application and its dependencies.
- PostgreSQL 15 or higher
- Rust 1.70 or higher
- Zig Compiler (for musl build)
- Linux Kernel 5.10 or higher (for optimal performance, but should work on older kernels)

## BUILD & RELEASE

This project using musl static library.

Before build, please install rust first.
Follow this [link](https://www.rust-lang.org/tools/install) to install rust.
Or run this command in the terminal:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Build

### Build with musl

### Prerequisites
Build with musl will embed into binary, and the application does not need glibc.

Using zig compiler, skip this step if you already have zig compiler installed.
Installing zig compiler:

Follow this [https://ziglang.org/learn/getting-started/#managers](https://ziglang.org/learn/getting-started/#managers) to install zig compiler.

If you using ubuntu, you can run this command:

```bash
sudo snap install zig --classic --beta
```
or using an older version of zig compiler:

```bash
sudo apt install zig
```

After installing zig compiler please install `cargo-zigbuild`

```bash
cargo install cargo-zigbuild
```
### Build

```bash
cargo zigbuild --target x86_64-unknown-linux-musl --release
```

If failed, try to use this command:

Clean the cargo cache:
```bash
cargo clean
```

Add safe flags to the build:

```bash
CXXFLAGS="-D_FILE_OFFSET_BITS=64 -Dstat64=stat -Dfstat64=fstat -mavx -mavx2 -mavx512f -mavx512bw -mavx512dq -mavx512vl -mevex512" \
cargo zigbuild --target x86_64-unknown-linux-musl --release
```


If Ruy is error try to add this flag:
`CMAKE_ARGS="-DWITH_RUY=OFF"`

Using ubuntu 20.04 compatible:

```bash
cargo zigbuild --target x86_64-unknown-linux-gnu.2.31 --release
```

Or using safe flags:

```bash
CXXFLAGS="-D_FILE_OFFSET_BITS=64 -Dstat64=stat -Dfstat64=fstat -mavx -mavx2 -mavx512f -mavx512bw -mavx512dq -mavx512vl -mevex512" \
cargo zigbuild --target x86_64-unknown-linux-gnu.2.31 --release
```

### Build with glibc

Using glibc will require glibc installed on your system (commonly available on most linux distributions).
If you want to use glibc, please run this command:
(this command will build a binary with glibc)

```bash
cargo build --release
```

## VARIABLES
- `TN_ROOT_DIR` define the root directory of tray node.
- `TN_NO_LIMIT` increase default limit than `u16` -> `65535` to `1048576` -> `TN_NO_LIMIT=true`
- `TN_THEMES_DIR` define the directory of themes.
- `TN_CONFIG_FILE` define default the path of a config file (extension should be `yaml` or `yml`)

# COMMANDS

The binary is located in the root directory of the project.
Make sure you are in the root directory of the project.

## Initialize a new config file

Run this command in the terminal:

```bash
./tnode init
```

And check a generated config file in the root directory.

## Starting the application server

Run the application server: (foreground)

```bash
./tnode server start
```

Run the application server in the background:

```bash
./tnode server start --daemonize
```

## Monitoring the application server

```bash
./tnode monitor
```

## Stopping the application server

```bash
./tnode server stop
```

## Reloading the application server

```bash
./tnode server reload
```

## Restarting the application server

```bash
./tnode server restart
```

This command will stop and start the application server in the background.

## NOTES

If you want to enable port below `1024` (eg: 80, 443), you can enable `CAP_NET_BIND_SERVICE` capability.
(`tnode` is path to the binary)

```bash
sudo setcap cap_net_bind_service=+ep tnode
```

Grepping where file that if in daemonize mode:

```bash
ps aux | grep tnode | awk '{print $2}' | xargs -I{} ls -l /proc/{}/exe 2>/dev/null
```

# STRUCTURES

```txt
/ (Project Root)
    ├── .cargo/ (Rust Cargo Configuration Directory)
    │   └── config.toml (Cargo Configuration)
    ├── external/ (External Data - Not Included)
    │   ├── linux-conf/ (Linux Configuration Sample)
    │   └── schema/ (Schema Directory)
    │       ├── theme.json (Theme Schema)
    │       └── *.json
    ├── src/ (Source Code)
    │       ├── app/ (Application Code)
    │       ├── core/ (Core Code)
    │       ├── factory/ (Factory Code)
    │       └── main.rs (Main Entry Point)
    ├── themes/ (Themes Directory)
    │       └── */ (theme list contain theme.yaml and any assets)
    ├── resources/ (Resources Internal Assets / Libraries)
    │   ├── i18n/ (Internationalization)
    │   ├── idna/ (IDNA Library)
    │   └── */ (Other Internal Assets / Libraries)
    ├── .gitignore (Git Ignore File)
    ├── build.rs (Build Script)
    ├── Cargo.lock (Cargo Lock File)
    ├── Cargo.toml (Cargo Crates Dependencies File)
    ├── config.example.yaml (Application Configuration Example)
    ├── LICENSE (License File)
    ├── README.md (Readme File)
    └── TODO.md (Todo File)

```

# LICENSE

This software is under [Apache License 2.0](LICENSE) (the "License"); you may not use this file except in compliance with the License. You may get a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0
