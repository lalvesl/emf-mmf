pub const fn phase_letter(phase: usize) -> char {
    (b'A' + (phase % 6) as u8) as char
}
