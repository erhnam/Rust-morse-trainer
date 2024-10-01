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
use wiringx::{
    gpio::{Input, Output, Value},
    platform::Platform,
    WiringX,
};

use std::time::Instant;
use std::{thread, time::Duration};

mod morse;

const END_CODE: u128 = 2000;
const PIN_LED: i32 = 25;
const PIN_BUTTON: i32 = 22;
const PIN_BUZZER: i32 = 15;

/*
BCM  -  Wiring X
 1   -     0
 2   -     1
 4   -     2
 5   -     3
 6   -     4
 7   -     5
 9   -     6
10   -     7
11   -     8
12   -     9
14   -     10
15   -     11
16   -     12
17   -     13
19   -     14
20   -     15
21   -     16
22   -     17
24   -     18
25   -     19
26   -     20
27   -     21
29   -     22
30   -     23 RUN
LED   -    25
31   -     26
32   -     27
*/

#[tracing::instrument]
fn main() -> anyhow::Result<()> {

    let wiringx = WiringX::new(Platform::MilkVDuo).unwrap();

    let subscriber = tracing_subscriber::FmtSubscriber::new();
    if tracing::subscriber::set_global_default(subscriber).is_err() {
        tracing::error!("Can't set global tracing::subscriber default");
    }

    tracing::info!("Morse Code");

    let led = wiringx.gpio_pin::<Output>(PIN_LED).unwrap();
    let button = wiringx.gpio_pin::<Input>(PIN_BUTTON).unwrap();
    let buzzer = wiringx.gpio_pin::<Output>(PIN_BUZZER).unwrap();

    let mut time_button_pressed: Option<Instant> = None;
    let mut time_end_secuence: Option<Instant> = None; // Detect end transmission
    let mut time_button_released: Option<Instant> = None;
    let mut pulse_info: Vec<morse::PulseInfo> = Vec::new();
    let mut total_press_time: u128 = morse::TOTAL_PRESSES_TIME; // Tiempo total de pulsación acumulado
    let mut total_presses: u128 = morse::TOTAL_PRESS; // Número total de pulsaciones

    loop {
        if button.read() == Value::High {
            if time_button_pressed.is_none() {
                time_button_pressed = Some(Instant::now());
                buzzer.write(Value::High);
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

            led.write(Value::High);
        } else {
            if time_button_released.is_none() {
                time_button_released = Some(Instant::now());
                buzzer.write(Value::Low);
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
            led.write(Value::Low);
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
