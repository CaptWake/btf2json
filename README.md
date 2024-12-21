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


## Volatility Patch

Volatility uses a JSON schema to validate profiles before using them. In order
for a `btf2json`-generated profile to pass the validation stage you need the
following patch:

```patch
diff --git a/volatility3/schemas/schema-6.2.0.json b/volatility3/schemas/schema-6.2.0.json
index 1f388005..65a6f5c6 100644
--- a/volatility3/schemas/schema-6.2.0.json
+++ b/volatility3/schemas/schema-6.2.0.json
@@ -105,7 +105,7 @@
       "properties": {
         "kind": {
           "type": "string",
-          "pattern": "^(dwarf|symtab|system-map)$"
+          "pattern": "^(btf|symdb|dwarf|symtab|system-map)$"
         },
         "name": {
           "type": "string"
```


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


## Packaging

Users of Arch-based distros can install `btf2json` via the
[AUR](https://aur.archlinux.org/packages/btf2json).


## Binary Releases

With each release we publish pre-compiled `btf2json` binaries for x64 Linux
systems. Running gigantic, statically-linked binaries downloaded from shady
sources is general a Bad Idea, but you can of course trust me, pinky swear :)
