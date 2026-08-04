/// Single-letter label for a phase: A, B, C, …
///
/// Wraps at 26 rather than at the phase count, so no two phases of a legal
/// configuration ever share a letter.
pub const fn phase_letter(phase: usize) -> char {
    (b'A' + (phase % 26) as u8) as char
}
