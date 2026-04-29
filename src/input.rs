/// Abstraction over any binary button/key input.
/// Returns 1 when pressed/held, 0 when released.
pub trait ButtonInput: Send {
    fn get_value(&self) -> u8;
}

// ─────────────────────────────────────────────────────────────────────────────
// GPIO implementation (only compiled with --features gpio)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "gpio")]
pub mod gpio {
    use super::ButtonInput;
    use linux_embedded_hal::sysfs_gpio::Pin;

    pub struct GpioButton {
        pin: Pin,
    }

    impl GpioButton {
        pub fn new(pin: Pin) -> Self {
            Self { pin }
        }
    }

    impl ButtonInput for GpioButton {
        fn get_value(&self) -> u8 {
            self.pin.get_value().unwrap_or(0)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Keyboard (spacebar) implementation (only compiled with --features keyboard)
//
// Uses `evdev` to read raw kernel input events so we get accurate key-down /
// key-up timestamps without relying on terminal echo or key-repeat.
// A background thread watches the device and stores the current state in an
// AtomicU8 so the main polling loop can read it without blocking.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "keyboard")]
pub mod keyboard {
    use super::ButtonInput;
    use evdev::{EventSummary, KeyCode};
    use std::sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    };

    pub struct KeyboardButton {
        /// 1 = SPACE currently held down, 0 = released.
        state: Arc<AtomicU8>,
        /// Set to false by the Ctrl+C handler; the event thread exits when it
        /// sees this, which drops the Device and releases /dev/input cleanly.
        running: Arc<AtomicBool>,
    }

    impl KeyboardButton {
        /// Scans `/dev/input/event*` for the first device that exposes
        /// `KEY_SPACE`, then spawns a background thread to track its state.
        pub fn new() -> anyhow::Result<Self> {
            let (device_path, _) = evdev::enumerate()
                .find(|(_, dev)| {
                    dev.supported_keys()
                        .map_or(false, |keys| keys.contains(KeyCode::KEY_SPACE))
                })
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "No keyboard device with KEY_SPACE found. \
                         Make sure you are in the 'input' group: \
                         sudo usermod -aG input $USER"
                    )
                })?;

            tracing::info!("Using keyboard device: {}", device_path.display());
            tracing::info!("Press Ctrl+C to exit cleanly.");

            let device = evdev::Device::open(&device_path)?;

            let state = Arc::new(AtomicU8::new(0));
            let running = Arc::new(AtomicBool::new(true));

            let state_bg = Arc::clone(&state);
            let running_bg = Arc::clone(&running);

            std::thread::spawn(move || {
                let mut dev = device;
                while running_bg.load(Ordering::Relaxed) {
                    match dev.fetch_events() {
                        Ok(events) => {
                            for ev in events {
                                // EventSummary::Key(InputEvent, KeyCode, value)
                                // value: 0 = up, 1 = down, 2 = repeat (ignored)
                                if let EventSummary::Key(_, code, value) = ev.destructure() {
                                    if code == KeyCode::KEY_SPACE {
                                        match value {
                                            1 => {
                                                state_bg.store(1, Ordering::Relaxed);
                                                tracing::debug!("KEY_SPACE down");
                                            }
                                            0 => {
                                                state_bg.store(0, Ordering::Relaxed);
                                                tracing::debug!("KEY_SPACE up");
                                            }
                                            _ => {} // auto-repeat
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("evdev read error: {e}");
                            break;
                        }
                    }
                }
                // Device is dropped here → fd closed → kernel releases everything
                tracing::debug!("Keyboard event thread exiting");
            });

            Ok(Self { state, running })
        }
    }

    impl ButtonInput for KeyboardButton {
        fn get_value(&self) -> u8 {
            self.state.load(Ordering::Relaxed)
        }
    }

    impl Drop for KeyboardButton {
        /// Ensures the event thread stops if KeyboardButton is dropped for any
        /// reason (e.g. an error in main after construction).
        fn drop(&mut self) {
            self.running.store(false, Ordering::Relaxed);
        }
    }
}
