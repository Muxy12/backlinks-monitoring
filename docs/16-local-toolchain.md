# 16 — Local Rust toolchain on this Windows box

This documents the **machine-local** setup used to compile and test the Rust
workspace on the development machine, and the one non-obvious workaround it
needs. None of this is required in CI (CI is Linux, [ci.yml](../.github/workflows/ci.yml))
or for a normal MSVC-based Windows install — it is specific to the constrained
environment this project was bootstrapped in.

## Why GNU, not MSVC

The MSVC toolchain needs Visual Studio Build Tools (~4 GB). Disk was the binding
constraint here, so we use the **`x86_64-pc-windows-gnu`** host, which bundles
its own linker (~1.5 GB total):

```bash
rustup-init.exe -y --default-host x86_64-pc-windows-gnu \
    --profile minimal --default-toolchain 1.98.1 -c rustfmt -c clippy
```

`rust-toolchain.toml` pins the channel **host-agnostically** (`channel =
"1.98.1"`), so this GNU host is used here while CI's Linux host and any MSVC dev
machine each resolve the same channel to their own host. The committed file is
correct for everyone; only the local install differs.

## The workaround: a real assembler for `dlltool`

The rustup windows-gnu toolchain ships a `self-contained` binutils that is
**incomplete** — its `dlltool.exe` cannot spawn an assembler (`as`), so any
crate that generates a Windows import library (e.g. `getrandom` →
`bcryptprimitives.dll`, which nearly everything pulls in transitively) fails to
compile with:

```
error: error calling dlltool 'dlltool.exe': program not found        # (no `as`)
```

Fix: put a **real** MinGW-w64 `as.exe` + `dlltool.exe` on PATH — but **not**
`gcc`/`ld`, so rustc keeps using its own self-contained linker (a full MinGW gcc
on PATH gets picked as the linker and then fails for lack of its runtime libs).

These two tools (plus their DLL dependencies) were extracted from the
[WinLibs](https://winlibs.com/) UCRT build and placed at a stable location:

```
~/.backlio/mingw-bin/    # as.exe, dlltool.exe, + required *.dll  (~62 MB)
```

## Building

```bash
export PATH="$HOME/.cargo/bin:$HOME/.backlio/mingw-bin:$PATH"

cargo test  -p backlio-core            # the state machine — no DB needed
cargo fmt   -p backlio-core --check
cargo clippy -p backlio-core --all-targets -- -D warnings
```

The full-workspace build (`cargo build --workspace`) additionally pulls sqlx,
reqwest+rustls, the Stripe SDK and lettre; that `target/` is several GB and is
deferred until more disk is free. `crates/core` is pure logic with a tiny
dependency tree, so it builds and tests in a few hundred MB.

## Reproducing on a fresh machine

- **The clean way:** install MSVC Build Tools and use the default
  `x86_64-pc-windows-msvc` host — none of the `dlltool` workaround applies.
- **This box's way:** GNU host as above, plus the `~/.backlio/mingw-bin` tools
  on PATH. If that directory is missing (e.g. it was only in the ephemeral
  scratchpad), re-extract `as.exe`, `dlltool.exe` and the `*.dll` files from a
  WinLibs `mingw64/bin` into it.

`~/.cargo/config.toml` on this box carries a comment pointing back here. It does
**not** override PATH — forcing the child PATH can starve the self-contained
linker of system DLLs during the final link, so PATH is set per-shell instead.
