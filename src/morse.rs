#[derive(Debug)]
pub struct PulseInfo {
    pub status: u8,
    pub millis: u128,
}

pub const PULSE_HIGH :u8 = 1;
pub const PULSE_LOW: u8 = 0;
pub const TOTAL_PRESSES_TIME: u128 = 2000;
pub const TOTAL_PRESS: u128 = 10;

static MORSE_MAP: [(&str, &str); 37] = [
    (".-", "A"), ("-...", "B"), ("-.-.", "C"), ("-..", "D"), (".", "E"),
    ("..-.", "F"), ("--.", "G"), ("....", "H"), ("..", "I"), (".---", "J"),
    ("-.-", "K"), (".-..", "L"), ("--", "M"), ("-.", "N"), ("---", "O"),
    (".--.", "P"), ("--.-", "Q"), (".-.", "R"), ("...", "S"), ("-", "T"),
    ("..-", "U"), ("...-", "V"), (".--", "W"), ("-..-", "X"), ("-.--", "Y"),
    ("--..", "Z"), (".----", "1"), ("..---", "2"), ("...--", "3"),
    ("....-", "4"), (".....", "5"), ("-....", "6"), ("--...", "7"),
    ("---..", "8"), ("----.", "9"), ("-----", "0"), (" ", " "),
];

fn parse_to_text(morse_code: &Vec<String>) {
    let mut morse_parsed = String::new();

    for morse_letter in morse_code {
        let found = MORSE_MAP.iter().find(|&&(morse, _)| morse == morse_letter.as_str());
        if let Some(&(_, ch)) = found {
            morse_parsed.push_str(ch);
        }
    }

    tracing::info!("Morse code: ");

    for code in morse_code {
        tracing::info!("{}", code);
    }

    tracing::info!("{}",morse_parsed.trim().to_string());
}

pub fn analize_secuence(pulse_info: &[PulseInfo], total_presses: u128, total_press_time: u128) {

    println!("TOTAL PRESSES {} TOTAL PRESS TIME {}", total_presses, total_press_time);
    let wpm = (total_presses * 60) / (total_press_time / 50);
    let dot = (60 * 1000) / (50 * wpm);
    let dash = dot * 3; // Dash is 3 units of dot
    let word = dot * 7; // Word is 7 units of do
    let threshold = dot;

    let mut letter :String = String::new();
    let mut array_strings: Vec<String> = Vec::new();

    // Recorremos el vector desde el segundo elemento
    for pulse in pulse_info.iter().skip(1) {
        if pulse.status == PULSE_HIGH {
            if pulse.millis <= dot + threshold {
                letter.push('.');
            } else if pulse.millis > dot + threshold {  // Cambio aquí
                letter.push('-');
            }
        } else if pulse.status == PULSE_LOW {
            if pulse.millis <= dash {
                continue;
            } else if pulse.millis > dash && pulse.millis < word {
                array_strings.push(letter.clone());
                letter.clear();
            } else if pulse.millis >= word {
                array_strings.push(letter.clone());
                letter.clear();
                letter.push(' ');
                array_strings.push(letter.clone());
                letter.clear();
            }
        }
    }

    array_strings.push(letter.clone());
    letter.clear();
    parse_to_text(&array_strings.clone());
    array_strings.clear();
}

