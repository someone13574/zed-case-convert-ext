use std::ops::Range;

const KEYWORDS: &[&str] = &[
    "void",
    "true",
    "false",
    "auto",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "constexpr",
    "continue",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "explicit",
    "extern",
    "for",
    "friend",
    "goto",
    "if",
    "inline",
    "namespace",
    "new",
    "noexcept",
    "nullptr",
    "operator",
    "private",
    "protected",
    "public",
    "register",
    "restrict",
    "return",
    "sizeof",
    "static",
    "struct",
    "switch",
    "template",
    "this",
    "throw",
    "try",
    "typedef",
    "typename",
    "union",
    "using",
    "virtual",
    "volatile",
    "while",
    "as",
    "async",
    "await",
    "crate",
    "dyn",
    "fn",
    "impl",
    "in",
    "let",
    "loop",
    "match",
    "mod",
    "move",
    "mut",
    "pub",
    "ref",
    "self",
    "Self",
    "super",
    "trait",
    "type",
    "unsafe",
    "where",
    "and",
    "def",
    "del",
    "elif",
    "except",
    "finally",
    "from",
    "global",
    "import",
    "is",
    "lambda",
    "nonlocal",
    "not",
    "or",
    "pass",
    "raise",
    "with",
    "yield",
    "None",
    "True",
    "False",
];

const TYPES: &[&str] = &[
    "char", "short", "int", "long", "float", "double", "signed", "unsigned", "bool", "_Bool",
    "usize", "isize", "str", "String", "Vec", "Option", "Result", "Box", "BOOL", "BYTE", "WORD",
    "DWORD", "QWORD", "UINT", "ULONG", "HANDLE",
];

pub fn in_range(text: &str, byte_range: Range<usize>) -> Vec<(Range<usize>, &str)> {
    let mut chars = text[byte_range.start..]
        .char_indices()
        .map(|(byte_idx, ch)| (byte_idx + byte_range.start, ch))
        .peekable();

    // The token under the start of the selection is only partially selected
    if text[..byte_range.start]
        .chars()
        .next_back()
        .is_some_and(is_ident_char)
    {
        while chars.next_if(|&(_, ch)| is_ident_char(ch)).is_some() {}
    }

    let mut idents = Vec::new();
    while let Some((byte_start, first)) = chars.next() {
        if byte_start >= byte_range.end {
            break;
        }
        if !is_ident_char(first) {
            continue;
        }

        let mut byte_end = byte_start + first.len_utf8();
        while let Some((byte_idx, ch)) = chars.next_if(|&(_, ch)| is_ident_char(ch)) {
            byte_end = byte_idx + ch.len_utf8();
        }
        if byte_end > byte_range.end {
            break;
        }
        if !first.is_numeric() {
            idents.push((byte_start..byte_end, &text[byte_start..byte_end]));
        }
    }
    idents
}

pub fn is_reserved(ident: &str) -> bool {
    KEYWORDS.contains(&ident)
        || TYPES.contains(&ident)
        || ident.starts_with("__")
        || ident.ends_with("_t")
        || ["u", "i", "f"].iter().any(|prefix| {
            ident
                .strip_prefix(prefix)
                .is_some_and(|bits| !bits.is_empty() && bits.chars().all(|ch| ch.is_ascii_digit()))
        })
}

fn is_ident_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}
