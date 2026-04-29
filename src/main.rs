/**
 * Morse Code decoder
 * Para transmitir "HOLA" en código Morse, necesitas enviar
 * La secuencia de puntos (·) y rayas (-) que representan cada una de las letras.
 * Aquí está la representación de cada letra:
 * H: .... (cuatro puntos)
 * O: --- (tres rayas)
 * L: .-.. (punto, raya, punto, punto)
 * A: .- (punto, raya)
 *
 * Timing rules:
 *   Dot        = 1 unit
 *   Dash       = 3 units
 *   Symbol gap = 1 unit  (between dots/dashes of the same letter)
 *   Letter gap = 3 units
 *   Word gap   = 7 units
 *
 * Usage:
 *   --input gpio      (default) physical button via sysfs GPIO
 *   --input keyboard  spacebar on a standard Linux keyboard
 */
use std::time::Instant;
use std::{thread, time::Duration};

mod input;
mod morse;

#[cfg(feature = "gpio")]
use linux_embedded_hal::sysfs_gpio::{Direction, Pin};
#[cfg(feature = "gpio")]
use std::collections::HashMap;

/// After this many milliseconds without a press the sequence is decoded.
const END_OF_SEQUENCE_MS: u128 = 2000;

/// Minimum stable time (ms) before a state transition is accepted.
const DEBOUNCE_MS: u128 = 10;

#[cfg(feature = "gpio")]
const PIN_LED: u64 = 25;
#[cfg(feature = "gpio")]
const PIN_BUTTON: u64 = 22;
#[cfg(feature = "gpio")]
const PIN_BUZZER: u64 = 15;

// ─────────────────────────────────────────────────────────────────────────────
// Input mode
// ─────────────────────────────────────────────────────────────────────────────

enum InputMode {
    #[cfg(feature = "gpio")]
    Gpio,
    #[cfg(feature = "keyboard")]
    Keyboard,
}

fn parse_args() -> InputMode {
    let args: Vec<String> = std::env::args().collect();
    let mode = args
        .iter()
        .position(|a| a == "--input" || a == "-i")
        .and_then(|pos| args.get(pos + 1))
        .map(String::as_str)
        .unwrap_or("gpio");

    match mode {
        #[cfg(feature = "keyboard")]
        "keyboard" | "kb" => InputMode::Keyboard,
        #[cfg(feature = "gpio")]
        "gpio" => InputMode::Gpio,
        other => {
            eprintln!("Unknown input mode '{}'. Available: {}", other, valid_modes());
            std::process::exit(1);
        }
    }
}

fn valid_modes() -> &'static str {
    match (cfg!(feature = "gpio"), cfg!(feature = "keyboard")) {
        (true, true) => "'gpio', 'keyboard'",
        (true, false) => "'gpio'",
        (false, true) => "'keyboard'",
        _ => "(none — rebuild with --features gpio or keyboard)",
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GPIO helpers
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "gpio")]
fn gpio_get_pin(pin_num: u64) -> u64 {
    let pin_map: HashMap<u64, u64> = [
        (1, 508),
        (2, 509),
        (4, 378),
        (5, 377),
        (6, 371),
        (7, 372),
        (9, 375),
        (10, 374),
        (11, 373),
        (12, 370),
        (14, 425),
        (15, 426),
        (16, 496),
        (17, 497),
        (19, 494),
        (20, 495),
        (21, 503),
        (22, 504),
        (24, 502),
        (25, 505),
        (26, 507),
        (27, 506),
        (29, 356),
        (41, 440),
    ]
    .iter()
    .cloned()
    .collect();
    *pin_map.get(&pin_num).unwrap_or(&0)
}

// ─────────────────────────────────────────────────────────────────────────────
// Optional hardware outputs (LED + buzzer)
// The struct is empty in keyboard mode; set_active() becomes a no-op.
// ─────────────────────────────────────────────────────────────────────────────

struct HardwareOutput {
    #[cfg(feature = "gpio")]
    led: Pin,
    #[cfg(feature = "gpio")]
    buzzer: Pin,
}

impl HardwareOutput {
    fn set_active(&self, active: bool) {
        #[cfg(feature = "gpio")]
        {
            let v = u8::from(active);
            self.led.set_value(v).unwrap();
            self.buzzer.set_value(v).unwrap();
        }
        #[cfg(not(feature = "gpio"))]
        let _ = active;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────────

#[tracing::instrument]
fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    if tracing::subscriber::set_global_default(subscriber).is_err() {
        tracing::error!("Failed to set global tracing subscriber");
    }

    let mode = parse_args();

    let button: Box<dyn input::ButtonInput>;
    let hw: HardwareOutput;

    match mode {
        #[cfg(feature = "gpio")]
        InputMode::Gpio => {
            tracing::info!("Input mode: GPIO button");

            let led = Pin::new(gpio_get_pin(PIN_LED));
            led.export().expect("Failed to export LED pin");
            led.set_direction(Direction::Out).expect("Failed to set LED direction");
            led.set_value(0).unwrap();

            let buzzer = Pin::new(gpio_get_pin(PIN_BUZZER));
            buzzer.export().expect("Failed to export buzzer pin");
            buzzer
                .set_direction(Direction::Out)
                .expect("Failed to set buzzer direction");
            buzzer.set_value(0).unwrap();

            let btn_pin = Pin::new(gpio_get_pin(PIN_BUTTON));
            btn_pin.export().expect("Failed to export button pin");
            btn_pin
                .set_direction(Direction::In)
                .expect("Failed to set button direction");

            hw = HardwareOutput { led, buzzer };
            button = Box::new(input::gpio::GpioButton::new(btn_pin));
        }

        #[cfg(feature = "keyboard")]
        InputMode::Keyboard => {
            tracing::info!("Input mode: spacebar");
            tracing::info!("Hold SPACE to send dots/dashes. Release for 2 s to decode.");
            hw = HardwareOutput {};
            button = Box::new(input::keyboard::KeyboardButton::new()?);
        }
    }

    run_loop(button, hw)
}

// ─────────────────────────────────────────────────────────────────────────────
// Main polling loop — identical regardless of input source
// ─────────────────────────────────────────────────────────────────────────────

fn run_loop(button: Box<dyn input::ButtonInput>, hw: HardwareOutput) -> anyhow::Result<()> {
    let mut button_pressed_at: Option<Instant> = None;
    let mut button_released_at: Option<Instant> = None;
    let mut sequence_end_at: Option<Instant> = None;
    let mut pulse_info: Vec<morse::PulseInfo> = Vec::new();
    let mut total_press_time: u128 = morse::TOTAL_PRESSES_TIME;
    let mut total_presses: u128 = morse::TOTAL_PRESS;

    // Debounce state
    let mut last_stable_value: u8 = 0;
    let mut pending_value: u8 = 0;
    let mut pending_since: Option<Instant> = None;

    tracing::info!("Ready — waiting for input");

    loop {
        let raw_value = button.get_value();

        // ── Debounce ──────────────────────────────────────────────────────────
        if raw_value != last_stable_value {
            if raw_value != pending_value {
                pending_value = raw_value;
                pending_since = Some(Instant::now());
            } else if pending_since
                .map(|t| t.elapsed().as_millis() >= DEBOUNCE_MS)
                .unwrap_or(false)
            {
                last_stable_value = pending_value;
                pending_since = None;
            }
        } else {
            pending_since = None;
        }

        let button_value = last_stable_value;

        // ── Pressed ───────────────────────────────────────────────────────────
        if button_value == 1 {
            if button_pressed_at.is_none() {
                button_pressed_at = Some(Instant::now());
                hw.set_active(true);
            }

            if let Some(released) = button_released_at {
                let gap_ms = released.elapsed().as_millis();
                pulse_info.push(morse::PulseInfo {
                    status: morse::PULSE_LOW,
                    millis: gap_ms,
                });
                tracing::debug!("LOW gap: {}ms", gap_ms);
                button_released_at = None;
            }

        // ── Released ──────────────────────────────────────────────────────────
        } else {
            if button_released_at.is_none() {
                button_released_at = Some(Instant::now());
                hw.set_active(false);
            }

            if let Some(pressed) = button_pressed_at {
                let press_ms = pressed.elapsed().as_millis();
                pulse_info.push(morse::PulseInfo {
                    status: morse::PULSE_HIGH,
                    millis: press_ms,
                });
                tracing::debug!("HIGH pulse: {}ms", press_ms);

                total_press_time += press_ms;
                total_presses += 1;

                button_pressed_at = None;
                sequence_end_at = Some(Instant::now());
            }
        }

        // ── End-of-sequence ───────────────────────────────────────────────────
        if let Some(end_at) = sequence_end_at {
            if end_at.elapsed().as_millis() >= END_OF_SEQUENCE_MS {
                sequence_end_at = None;
                tracing::info!(
                    "Sequence complete — {} pulses, {}ms press time",
                    total_presses,
                    total_press_time
                );
                morse::analyze_sequence(&pulse_info, total_presses, total_press_time);
                pulse_info.clear();
                total_press_time = morse::TOTAL_PRESSES_TIME;
                total_presses = morse::TOTAL_PRESS;
            }
        }

        thread::sleep(Duration::from_millis(5));
    }
}
