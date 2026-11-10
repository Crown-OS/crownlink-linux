use std::borrow::Cow;

/// `text` safe to place in a notification body, which servers may read as markup.
pub fn escape(text: &str) -> Cow<'_, str> {
    if !text.contains(['&', '<', '>', '"', '\'']) {
        return Cow::Borrowed(text);
    }
    let mut escaped = String::with_capacity(text.len() + 16);
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }
    Cow::Owned(escaped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_borrowed() {
        assert_eq!(escape("tel:+1555"), Cow::Borrowed("tel:+1555"));
    }

    #[test]
    fn markup_characters_are_escaped() {
        assert_eq!(
            escape("<b>a&b</b> \"q\" 'r'"),
            "&lt;b&gt;a&amp;b&lt;/b&gt; &quot;q&quot; &#39;r&#39;"
        );
    }
}
