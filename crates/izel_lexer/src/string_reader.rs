use crate::cursor::Cursor;

/// Consumes an escape sequence from the cursor.
/// Assumes the leading backslash has already been consumed.
pub fn eat_escape(cursor: &mut Cursor<'_>) {
    match cursor.bump() {
        Some('n') | Some('r') | Some('t') | Some('\\') | Some('\'') | Some('"') | Some('0') => {}
        Some('u') if cursor.first() == '{' => {
            cursor.bump();
            cursor.eat_while(|c| c.is_ascii_hexdigit() || c == '_');
            if cursor.first() == '}' {
                cursor.bump();
            }
        }
        Some('x') if cursor.first().is_ascii_hexdigit() => {
            // \xHH hex escape
            cursor.bump();
            if cursor.first().is_ascii_hexdigit() {
                cursor.bump();
            }
        }
        _ => {
            // Unknown escape - the bump already consumed the char after '\'
        }
    }
}
