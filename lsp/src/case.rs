#[derive(Clone, Copy)]
pub enum Case {
    Snake,
    Camel,
    Pascal,
    ScreamingSnake,
}

impl Case {
    pub const ALL: [Case; 4] = [Case::Snake, Case::Camel, Case::Pascal, Case::ScreamingSnake];

    pub fn name(self) -> &'static str {
        match self {
            Case::Snake => "snake_case",
            Case::Camel => "camelCase",
            Case::Pascal => "PascalCase",
            Case::ScreamingSnake => "SCREAMING_SNAKE_CASE",
        }
    }

    pub fn convert(self, ident: &str) -> String {
        let trimmed = ident.trim_start_matches('_');
        let core = trimmed.trim_end_matches('_');
        if core.is_empty() {
            return ident.to_string();
        }
        let prefix = &ident[..ident.len() - trimmed.len()];
        let suffix = &trimmed[core.len()..];

        let words: Vec<String> = split_words(core)
            .into_iter()
            .enumerate()
            .map(|(word_idx, word)| match self {
                Case::Snake => word.to_lowercase(),
                Case::ScreamingSnake => word.to_uppercase(),
                Case::Camel if word_idx == 0 => word.to_lowercase(),
                Case::Camel | Case::Pascal => capitalize(word),
            })
            .collect();
        let separator = match self {
            Case::Snake | Case::ScreamingSnake => "_",
            Case::Camel | Case::Pascal => "",
        };
        format!("{prefix}{}{suffix}", words.join(separator))
    }
}

fn split_words(ident: &str) -> Vec<&str> {
    let mut words = Vec::new();
    for part in ident.split('_').filter(|part| !part.is_empty()) {
        let chars: Vec<(usize, char)> = part.char_indices().collect();
        let mut byte_start = 0;
        for char_idx in 1..chars.len() {
            let (byte_idx, ch) = chars[char_idx];
            let prev = chars[char_idx - 1].1;
            let next = chars.get(char_idx + 1).map(|&(_, ch)| ch);
            let boundary = ch.is_uppercase()
                && (prev.is_lowercase()
                    || prev.is_numeric()
                    || (prev.is_uppercase() && next.is_some_and(char::is_lowercase)));
            if boundary {
                words.push(&part[byte_start..byte_idx]);
                byte_start = byte_idx;
            }
        }
        words.push(&part[byte_start..]);
    }
    words
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect(),
        None => String::new(),
    }
}
