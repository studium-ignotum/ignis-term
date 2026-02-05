use nanoid::nanoid;

/// Characters for session codes - excludes 0/O/1/I/L to avoid confusion
const CODE_ALPHABET: [char; 31] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K',
    'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W',
    'X', 'Y', 'Z', '2', '3', '4', '5', '6', '7', '8', '9',
];

/// Generate a 6-character session code
pub fn generate_session_code() -> String {
    nanoid!(6, &CODE_ALPHABET)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_length() {
        let code = generate_session_code();
        assert_eq!(code.len(), 6);
    }

    #[test]
    fn test_code_alphabet() {
        let code = generate_session_code();
        for c in code.chars() {
            assert!(CODE_ALPHABET.contains(&c), "Invalid char: {}", c);
        }
    }

    #[test]
    fn test_no_confusing_chars() {
        // Generate many codes and verify none contain confusing chars
        for _ in 0..100 {
            let code = generate_session_code();
            assert!(!code.contains('0'));
            assert!(!code.contains('O'));
            assert!(!code.contains('1'));
            assert!(!code.contains('I'));
            assert!(!code.contains('L'));
        }
    }
}
