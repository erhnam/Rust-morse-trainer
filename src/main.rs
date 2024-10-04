/**
* Código Morse:
*
* Una letra puede contener hasta cuatro elementos (puntos o rayas).
* Por ejemplo, la letra "E" se representa con un solo punto,
* mientras que la letra "Q" se representa con cuatro elementos: "-.-."
*
* Para transmitir "HOLA" en código Morse, necesitas enviar
* la secuencia de puntos (·) y rayas (-) que representan cada una de las letras.
* Aquí está la representación de cada letra:
H: .... (cuatro puntos)
O: --- (tres rayas)
L: .-.. (punto, raya, punto, punto)
A: .- (punto, raya)
* Entonces, para enviar "HOLA" en código Morse, enviarías la siguiente secuencia:
* .... --- .-.. .-
*
* The length of a dot is 1 time unit.
A dash is 3 time units.
The space between symbols (dots and dashes) of the same letter is 1 time unit.
The space between letters is 3 time units.
The space between words is 7 time units
*/

use linux_embedded_hal::sysfs_gpio::{Direction, Pin};
use std::time::Instant;
use std::{thread, time::Duration};
use std::collections::HashMap;

mod morse;

const END_CODE: u128 = 2000;
const PIN_LED: u64 = 25;
const PIN_BUTTON: u64 = 22;
const PIN_BUZZER: u64 = 15;

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

#[tracing::instrument]
fn main() -> anyhow::Result<()> {

    let subscriber = tracing_subscriber::FmtSubscriber::new();

    if tracing::subscriber::set_global_default(subscriber).is_err() {
        tracing::error!("Can't set global tracing::subscriber default");
    }

    tracing::info!("Morse Code");

    let led = Pin::new(gpio_get_pin(PIN_LED));
    led.export().expect("error exporting cs pin");
    led.set_direction(Direction::Out).expect("error setting cs pin direction");
    led.set_value(0).unwrap();

    let buzzer = Pin::new(gpio_get_pin(PIN_BUZZER));
    buzzer.export().expect("error exporting cs pin");
    buzzer.set_direction(Direction::Out).expect("error setting cs pin direction");
    buzzer.set_value(0).unwrap();

    let button = Pin::new(gpio_get_pin(PIN_BUTTON));
    button.export().expect("error exporting cs pin");
    button.set_direction(Direction::In).expect("error setting cs pin direction");

    let mut time_button_pressed: Option<Instant> = None;
    let mut time_end_secuence: Option<Instant> = None; // Detect end transmission
    let mut time_button_released: Option<Instant> = None;
    let mut pulse_info: Vec<morse::PulseInfo> = Vec::new();
    let mut total_press_time: u128 = morse::TOTAL_PRESSES_TIME; // Tiempo total de pulsación acumulado
    let mut total_presses: u128 = morse::TOTAL_PRESS; // Número total de pulsaciones

    loop {
        if button.get_value().unwrap() == 1 {
            if time_button_pressed.is_none() {
                time_button_pressed = Some(Instant::now());
                buzzer.set_value(1).unwrap();
            }

            if let Some(released) = time_button_released {
                let elapsed_time = Instant::now().duration_since(released);
                let time_ms = elapsed_time.as_millis();
                pulse_info.push(morse::PulseInfo {
                    status: morse::PULSE_LOW,
                    millis: time_ms,
                });
                time_button_released = None;
            }

            led.set_value(1).unwrap();
        } else {
            if time_button_released.is_none() {
                time_button_released = Some(Instant::now());
                buzzer.set_value(0).unwrap();
            }
            if let Some(pressed) = time_button_pressed {
                let elapsed_time = Instant::now().duration_since(pressed);
                let time_ms = elapsed_time.as_millis();
                pulse_info.push(morse::PulseInfo {
                    status: morse::PULSE_HIGH,
                    millis: time_ms,
                });

                total_press_time += time_ms; // Actualizar el tiempo total de pulsación acumulado
                total_presses += 1; // Incrementar el número total de pulsaciones

                time_button_pressed = None;
                time_end_secuence = Some(Instant::now());
            }
            led.set_value(0).unwrap();
        }

        // Check if end transmission
        if let Some(end_secuence) = time_end_secuence {
            let elapsed_time = Instant::now().duration_since(end_secuence);
            let millis = elapsed_time.as_millis();
            if millis >= END_CODE {
                time_end_secuence = None;
                morse::analize_secuence(&pulse_info, total_presses, total_press_time);
                pulse_info.clear();
                total_press_time = morse::TOTAL_PRESSES_TIME; // Actualizar el tiempo total de pulsación acumulado
                total_presses = morse::TOTAL_PRESS; // Incrementar el número total de pulsaciones
            }
        }

        thread::sleep(Duration::from_millis(5));
    }
}
