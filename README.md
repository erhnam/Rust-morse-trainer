# Rust Morse Trainer

A Morse code decoder written in Rust. Supports two input modes: a physical button via GPIO (embedded board) or the spacebar on a standard Linux keyboard.

---

## Install dependencies

```bash
sudo apt-get install build-essential
sudo apt-get install libc6-dev-riscv64-cross
sudo apt-get install gcc-riscv64-linux-gnu
```

> [!NOTE]
> For keyboard mode, make sure your user is in the `input` group so you can read `/dev/input/event*` without root:
>
> ```bash
> sudo usermod -aG input $USER
> # Log out and back in for the change to take effect
> ```

---

## Build

### Keyboard mode (standard Linux PC)

```bash
cargo build --release --features keyboard
```

### GPIO mode (embedded board)

```bash
cargo build --release --features gpio
```

### Both modes available in the same binary

```bash
cargo build --release --features gpio,keyboard
```

---

## Run

### Keyboard mode — hold `SPACE` to transmit, release 2 s to decode

```bash
./target/release/morse --input keyboard
```

### GPIO mode — physical button on the board

```bash
./target/release/morse --input gpio
```

---

## How it works

Hold the button/key for a **short** time to send a dot `·`, hold longer for a dash `—`. The decoder calculates the dot/dash threshold automatically from your own rhythm, so no manual calibration is needed.

| Signal | Duration |
|--------|----------|
| Dot | 1 unit |
| Dash | 3 units |
| Gap between symbols | 1 unit |
| Gap between letters | 3 units |
| Gap between words | 7 units |

After **2 seconds** of silence the current sequence is decoded and printed.

---

## Logging

Set the `RUST_LOG` environment variable to control verbosity:

```bash
RUST_LOG=debug ./target/release/morse --input keyboard   # show every pulse
RUST_LOG=info  ./target/release/morse --input keyboard   # decoded text only
```
