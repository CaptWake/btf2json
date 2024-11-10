# `btf2json`

Leverage the
[BPF Type Format (BTF)](https://www.kernel.org/doc/html/latest/bpf/btf.html) to
generate [Volatility 3](https://github.com/volatilityfoundation/volatility3)
profiles!

`btf2json` can generate Volatility 3 profiles from a stripped Linux kernel,
i.e., without debugging information, and its `System.map` file. For many systems,
this means collecting the `vmlinuz` boot image and System.map from the boot
partition of the target system,
[extracting `vmlinuz`](https://github.com/torvalds/linux/blob/master/scripts/extract-vmlinux),
and running `btf2json` like this:

```shell
$ btf2json --btf vmlinux --map System.map
# prints profile to stdout
```

gets you a working profile that can be used to analyze a memory image of the
system with Volatility 3.

Interested in how this works? See the
[blog post](https://blog.eb9f.de/2024/11/10/btf2json.html) for all those juicy
technical details.


## Building

`btf2json` is written in [Rust](https://www.rust-lang.org/). The easiest way
to install the Rust toolchain on your system is via
[`rustup`](https://rustup.rs/).

```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

(Alternatively, many distributions provide `rustup` and Rust via their package
repositories.)

You can now use `cargo`, Rust's package manager, to compile this project and
install the executable to your PATH:

```shell
cargo install --locked --path .
```

If you just want to build the final executable run:

```shell
cargo build --release --locked
```

The executable is located at `./target/release/btf2json`.
