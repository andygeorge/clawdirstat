# `clawdirstat`

a terminal disk usage visualizer, like KDirStat/WinDirStat but for your shell.

## usage

```sh
clawdirstat [DIR] [-n <count>]
```

- `DIR` — directory to scan (defaults to cwd)
- `-n` — limit top-level folders shown

navigate with `j`/`k` or arrow keys, `q` to quit.

## install

```sh
cargo install --git https://github.com/andygeorge/clawdirstat
```

## build from source

```sh
cargo build --release
cargo install --path .
```

## stack

- [ratatui](https://github.com/ratatui-org/ratatui) — TUI rendering
- [clap](https://github.com/clap-rs/clap) — CLI arg parsing
- [rusqlite](https://github.com/rusqlite/rusqlite) — scan result caching (SQLite)
