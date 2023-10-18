pub fn to_snake(s: &String) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    let chars: Vec<char> = s.chars().collect();

    for i in 0..chars.len() {
        let current = chars[i];

        // If the current character is uppercase
        if current.is_ascii_uppercase() {
            // Insert an underscore if the next character is lowercase or at the end of the string.
            if i != 0
                && i + 1 < chars.len()
                && chars[i + 1].is_ascii_lowercase()
                && (!result.ends_with('_'))
            {
                result.push('_');
            }

            // Insert an underscore if it's preceded by a lowercase and doesn't already have one before it.
            if i != 0 && chars[i - 1].is_ascii_lowercase() && !result.ends_with('_') {
                result.push('_');
            }
        }

        result.push(current);
    }

    result.to_ascii_lowercase()
}
