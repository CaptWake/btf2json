# Memory Forensics on Android with Volatility3

This guide walks you through acquiring a memory dump from an Android device/emulator and generating a custom Volatility3 profile using `lemon` and `btf2json`.

---

## 📥 Step 1: Download and Upload `lemon`

1. Download the appropriate `lemon` binary from the [lemon GitHub Releases](https://github.com/eurecom-s3/lemon/releases/tag/v1.2), matching your Android architecture.
2. Upload `lemon` to your device/emulator:

```bash
adb push <lemon_binary> /data/local/tmp
```

---

## Step 2: Enter the Android Shell as Root

```bash
adb shell
su
```

---

## Step 3: Dump Required Files

From the Android shell, collect kernel symbols, BTF info, and memory:

```bash
cd /data/local/tmp
echo 0 > /proc/sys/kernel/kptr_restrict
cat /proc/kallsyms > kallsyms
cat /sys/kernel/btf/vmlinux > btf_symb
./<lemon_binary> -d mem_dump
```

---

## 💻 Step 4: Transfer Files to Host

Exit the shell and pull the files to your analysis workstation:

```bash
adb pull /data/local/tmp/kallsyms
adb pull /data/local/tmp/btf_symb
adb pull /data/local/tmp/mem_dump
```

---

## 🏗️ Step 5: Build `btf2json` and Generate Profile

### Install Rust (if not already installed):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Clone and build `btf2json`:

```bash
git clone https://github.com/CaptWake/btf2json.git
cd btf2json
cargo build --release
```

### Generate Volatility3 Profile:

```bash
# Replace <android banner> with output of `uname -a` from Android shell
# Replace <android architecture> with `x86_64` or `arm64`
./target/release/btf2json \
  --map ../kallsyms \
  --btf ../btf_symb \
  --arch <android architecture> \
  --banner "<android banner>" > profile.json

python utilities/patch_profile.py -f ./profile.json
```

> ❗ **Expected Warning**:
>
> ```
> [ERROR btf2json::isf] 4 symbols reference missing types, 4 unique types are missing
> ```
>
> This is **normal** and corrected by `patch_profile.py`.

---

## 🛠️ Step 6: Patch Volatility3 to Support BTF Profiles

Volatility3 uses a strict JSON schema to validate profiles, so we need to patch the schema.

### Apply the Patch

1. **Clone the Volatility3 repository (if not already cloned):**

   ```bash
   git clone https://github.com/volatilityfoundation/volatility3.git
   cd volatility3
   ```

2. **Create the patch file:**

   Create a file named `btf_support.patch` with the following content:

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

3. **Apply the patch with Git:**

   ```bash
   git apply btf_support.patch
   ```

> After this, Volatility3 will accept the profiles generated using `btf2json`.


---

## Step 7: Use Volatility3 with Android Memory Dump

Run your Volatility3 plugin of choice:

```bash
./vol.py -s <path to profile.json> -f <path to mem_dump> <plugin>
```

>  **Tip**: Add `-vvvvvvv` to increase verbosity for plugin debugging.

---

## Troubleshooting: Unknown Banner

If you don’t know the kernel banner string, try:

```bash
./vol.py -f <path to mem_dump> banner
```

Use this output as the value for `--banner` in Step 5.


