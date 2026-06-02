//! Bilingual naming: Devanagari names are canonical; Latin (and alternate
//! Sanskrit) spellings are aliases that canonicalise to them. This lets the
//! same program be written keyboard-friendly Latin or in Devanagari.

/// (alias, canonical) pairs for ordinary identifiers (builtins & stdlib).
///
/// Note: operator *words* like `add` map to the operator symbol the engine
/// understands (`+`), which infix sugar also produces.
const IDENT_ALIASES: &[(&str, &str)] = &[
    // booleans / unit / lists
    ("true", "सत्य"),
    ("false", "असत्य"),
    ("unit", "एकक"),
    ("cons", "युग्म"),
    ("nil", "रिक्त"),
    ("none", "रिक्त"),
    // arithmetic (word spellings → operator symbol)
    ("add", "+"),
    ("योग", "+"),
    ("sub", "-"),
    ("वियोग", "-"),
    ("mul", "*"),
    ("गुणन", "*"),
    ("div", "/"),
    ("भाग", "/"),
    ("mod", "%"),
    ("शेष", "%"),
    // comparison
    ("eq", "=="),
    ("तुल्य", "=="),
    ("ne", "!="),
    ("lt", "<"),
    ("le", "<="),
    ("gt", ">"),
    ("ge", ">="),
    // logic
    ("not", "न"),
    ("and", "च"),
    ("or", "वा"),
    // string / list builtins
    ("concat", "++"),
    ("संयोग", "++"),
    ("len", "दीर्घ"),
    ("length", "दीर्घ"),
    ("to_str", "रूप"),
    ("show", "रूप"),
    ("substr", "अंश"),
    ("chars", "अक्षर"),
    // higher-order stdlib
    ("map", "प्रति"),
    ("filter", "छन्न"),
    ("fold", "संहार"),
    ("reverse", "विपर्यय"),
    ("append", "योजन"),
    ("head", "शीर्ष"),
    ("tail", "पुच्छ"),
    ("member", "सदस्य"),
    ("range", "श्रेणी"),
    ("sum", "समष्टि"),
    // io actions
    ("pure", "शुद्ध"),
    ("return", "शुद्ध"),
    ("bind", "बन्ध"),
    ("print", "मुद्रण"),
    ("println", "मुद्रण"),
    ("read", "पठन"),
    ("read_line", "पठन"),
    ("seq", "अनुक्रम"),
    ("main", "मुख्य"),
    ("error", "दोष"),
];

/// Canonicalise an identifier (used by the parser on every `Ident`).
pub fn canonical(name: &str) -> String {
    for (alias, canon) in IDENT_ALIASES {
        if *alias == name {
            return (*canon).to_string();
        }
    }
    name.to_string()
}

/// Convert Arabic digits in a string to Devanagari (for default printing).
pub fn devanagari_digits(s: &str) -> String {
    s.chars()
        .map(|c| {
            c.to_digit(10)
                .and_then(|d| char::from_u32(0x0966 + d))
                .unwrap_or(c)
        })
        .collect()
}
