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
use sysfs_gpio::{Direction, Pin};
use std::time::Instant;
use std::thread;
use std::time::Duration;

mod morse;

const END_CODE :u128 = 2000;
const PIN_LED: u64 = 41;
const PIN_BUTTON: u64 = 29;
const PIN_BUZZER: u64 = 20;
const LED_OFF: u8 = 0;
const LED_ON: u8 = 1;

fn gpio_get_pin(pin_num: u64) -> u64 {
    let pinnum = [1, 2, 4, 5, 6, 7, 9, 10, 11, 12, 14, 15, 16, 17, 19, 20, 21, 22, 24, 25, 26, 27, 29, 41];
    let pinmap = [508, 509, 378, 377, 371, 372, 375, 374, 373, 370, 425, 426, 496, 497, 494, 495, 503, 504, 502, 505, 507, 506, 356, 440];

    match pinnum.iter().position(|&x| x == pin_num) {
        Some(index) => pinmap[index],
        None => 0, // Valor predeterminado si el pin no se encuentra en la lista
    }
}

fn configure_pins(led: Pin, button: Pin, buzzer: Pin) {
    /* Exported pins */
    led.export().unwrap();
    button.export().unwrap();
    buzzer.export().unwrap();

    /* Configure Pin directions */
    led.set_direction(Direction::Out).unwrap();
    button.set_direction(Direction::In).unwrap();
    buzzer.set_direction(Direction::Out).unwrap();
}

fn play_sound(buzzer: Pin) {
    thread::spawn(move || {
        let _ = buzzer.set_value(1);
    });
}

fn stop_sound(buzzer: Pin) {
    thread::spawn(move || {
        let _ = buzzer.set_value(0);
    });
}

#[tokio::main(flavor = "current_thread")]
#[tracing::instrument]
async fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    tracing::info!("Morse Code");

    let led: Pin = Pin::new(gpio_get_pin(PIN_LED));
    let button: Pin = Pin::new(gpio_get_pin(PIN_BUTTON));
    let buzzer: Pin = Pin::new(gpio_get_pin(PIN_BUZZER));

    configure_pins(led, button, buzzer);

    let mut time_button_pressed: Option<Instant> = None;
    let mut time_end_secuence: Option<Instant> = None; // Detect end transmission
    let mut time_button_released: Option<Instant> = None;
    let mut pulse_info: Vec<morse::PulseInfo> = Vec::new();
    let mut total_press_time: u128 = morse::TOTAL_PRESSES_TIME; // Tiempo total de pulsación acumulado
    let mut total_presses: u128 = morse::TOTAL_PRESS; // Número total de pulsaciones

    loop {
        if button.get_value()? == LED_ON {
            if time_button_pressed.is_none() {
                time_button_pressed = Some(Instant::now());
                play_sound(buzzer.clone());
            }

            if let Some(released) = time_button_released {
                let elapsed_time = Instant::now().duration_since(released);
                let time_ms = elapsed_time.as_millis();
                pulse_info.push(
                    morse::PulseInfo {
                        status: morse::PULSE_LOW,
                        millis: time_ms,
                    });
                time_button_released = None;
            }

            led.set_value(LED_ON)?;
        } else {
            if time_button_released.is_none() {
                time_button_released = Some(Instant::now());
                stop_sound(buzzer.clone());
            }
            if let Some(pressed) = time_button_pressed {
                let elapsed_time = Instant::now().duration_since(pressed);
                let time_ms = elapsed_time.as_millis();
                pulse_info.push(
                    morse::PulseInfo {
                        status: morse::PULSE_HIGH,
                        millis: time_ms,
                    });
                    

                total_press_time += time_ms; // Actualizar el tiempo total de pulsación acumulado
                total_presses += 1; // Incrementar el número total de pulsaciones

                time_button_pressed = None;
                time_end_secuence = Some(Instant::now());
            }
            led.set_value(LED_OFF)?;
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
