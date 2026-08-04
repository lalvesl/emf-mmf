/// Single-letter label for a phase: A, B, C, …
const ALPHABET_SIZE: usize = 'Z' as usize - 'A' as usize;

pub const fn phase_letter(phase: usize) -> char {
    (b'A'
        + (if crate::config::MAX_PHASES > ALPHABET_SIZE {
            phase % ALPHABET_SIZE
        } else {
            phase
        }) as u8) as char
}
