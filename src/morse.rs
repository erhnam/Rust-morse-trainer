#[derive(Debug)]
pub struct PulseInfo {
    pub status: u8,
    pub millis: u128,
}

pub const PULSE_HIGH: u8 = 1;
pub const PULSE_LOW: u8 = 0;
pub const TOTAL_PRESSES_TIME: u128 = 0;
pub const TOTAL_PRESS: u128 = 0;

static MORSE_MAP: [(&str, &str); 37] = [
    (".-", "A"),
    ("-...", "B"),
    ("-.-.", "C"),
    ("-..", "D"),
    (".", "E"),
    ("..-.", "F"),
    ("--.", "G"),
    ("....", "H"),
    ("..", "I"),
    (".---", "J"),
    ("-.-", "K"),
    (".-..", "L"),
    ("--", "M"),
    ("-.", "N"),
    ("---", "O"),
    (".--.", "P"),
    ("--.-", "Q"),
    (".-.", "R"),
    ("...", "S"),
    ("-", "T"),
    ("..-", "U"),
    ("...-", "V"),
    (".--", "W"),
    ("-..-", "X"),
    ("-.--", "Y"),
    ("--..", "Z"),
    (".----", "1"),
    ("..---", "2"),
    ("...--", "3"),
    ("....-", "4"),
    (".....", "5"),
    ("-....", "6"),
    ("--...", "7"),
    ("---..", "8"),
    ("----.", "9"),
    ("-----", "0"),
    (" ", " "),
];

/// Returns (dot_ms, dot_dash_threshold_ms).
///
/// dot_ms                — duration of the shortest press (= 1 Morse unit).
/// dot_dash_threshold_ms — anything shorter is a dot, anything longer is a dash.
///
/// When only dots are present the threshold is set to 2× dot so a future dash
/// would still be recognised.  When both dots and dashes are present the
/// threshold is the midpoint between the two clusters, giving the most
/// tolerance regardless of the sender's speed.
fn calculate_dot_duration(pulse_info: &[PulseInfo]) -> (u128, u128) {
    let mut highs: Vec<u128> = pulse_info
        .iter()
        .filter(|p| p.status == PULSE_HIGH)
        .map(|p| p.millis)
        .collect();

    if highs.is_empty() {
        tracing::warn!("No HIGH pulses found; falling back to defaults (dot=100ms)");
        return (100, 200);
    }

    highs.sort();

    let min_pulse = highs[0];
    let max_pulse = highs[highs.len() - 1];

    tracing::debug!(
        "HIGH pulses — min={}ms  max={}ms  count={}",
        min_pulse,
        max_pulse,
        highs.len()
    );

    if max_pulse <= min_pulse * 2 {
        // All pulses similar length → treat as dots only.
        // Threshold at 2× dot so a dash would still be caught later.
        let dot = min_pulse;
        (dot, dot * 2)
    } else {
        // Two clusters (dots + dashes) clearly visible.
        // dot  = shortest press  (centroid of the dot cluster)
        // threshold = midpoint between the two clusters
        let dot = min_pulse;
        let threshold = (min_pulse + max_pulse) / 2;
        tracing::debug!("Dot/dash threshold: {}ms", threshold);
        (dot, threshold)
    }
}

fn parse_to_text(morse_code: &[String]) {
    let mut decoded = String::new();

    for morse_letter in morse_code {
        let found = MORSE_MAP.iter().find(|&&(morse, _)| morse == morse_letter.as_str());

        match found {
            Some(&(_, ch)) => decoded.push_str(ch),
            None if !morse_letter.is_empty() => {
                tracing::warn!("Unknown morse symbol: '{}'", morse_letter);
            }
            _ => {}
        }
    }

    tracing::info!("--- Decoded sequence ---");
    tracing::info!("Morse symbols: {:?}", morse_code);
    tracing::info!("Decoded text : {}", decoded.trim());
}

pub fn analyze_sequence(pulse_info: &[PulseInfo], _total_presses: u128, _total_press_time: u128) {
    if pulse_info.is_empty() {
        return;
    }

    let (dot, dot_dash_threshold) = calculate_dot_duration(pulse_info);

    // Umbrales sugeridos:
    // Entre símbolos: < 2u
    // Entre letras: 2u a 5u
    // Entre palabras: > 5u
    // Definimos los umbrales basados en el 'dot' detectado
    let intra_letter_limit = dot * 3;
    let word_gap_limit = dot * 7; // Siguiendo más fielmente el estándar para espacios.

    let mut letter = String::new();
    let mut symbols: Vec<String> = Vec::new();

    for pulse in pulse_info.iter() {
        if pulse.status == PULSE_HIGH {
            if pulse.millis <= dot_dash_threshold {
                letter.push('.');
            } else {
                letter.push('-');
            }
        } else {
            let gap = pulse.millis;

            if gap >= word_gap_limit {
                // ESPACIO ENTRE PALABRAS
                if !letter.is_empty() {
                    symbols.push(letter.drain(..).collect());
                }
                symbols.push(" ".to_string());
            } else if gap >= intra_letter_limit {
                // ESPACIO ENTRE LETRAS
                if !letter.is_empty() {
                    symbols.push(letter.drain(..).collect());
                }
            }
            // Si el silencio es menor a intra_letter_limit (ej. 2.9 * dot),
            // NO cerramos la letra actual, permitiendo que la "O" se agrupe.
        }
    }

    if !letter.is_empty() {
        symbols.push(letter);
    }

    parse_to_text(&symbols);
}
