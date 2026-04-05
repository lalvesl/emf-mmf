pub const fn phase_letter(phase: usize) -> char {
    ('A' as u8 + (phase % 6) as u8) as char
}
