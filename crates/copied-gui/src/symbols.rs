pub struct SymbolGroup {
    pub name: &'static str,
    pub symbols: &'static [&'static str],
}

pub const CATALOG: &[SymbolGroup] = &[
    SymbolGroup {
        name: "Matemática",
        symbols: &[
            "±", "×", "÷", "≈", "≠", "≤", "≥", "∞", "√", "∑", "∏", "∫", "π", "°",
        ],
    },
    SymbolGroup {
        name: "Setas",
        symbols: &["→", "←", "↑", "↓", "↔", "⇒", "⇐", "⇔"],
    },
    SymbolGroup {
        name: "Ícones gerais",
        symbols: &["★", "☆", "✓", "✗", "©", "®", "™", "•", "…", "§"],
    },
    SymbolGroup {
        name: "Números Especiais",
        symbols: &[
            // Frações
            "¼", "½", "¾", "⅓", "⅔", "⅕", "⅖", "⅗", "⅘", "⅙", "⅚", "⅛", "⅜", "⅝", "⅞",
            // Numerais romanos (maiúsculos)
            "Ⅰ", "Ⅱ", "Ⅲ", "Ⅳ", "Ⅴ", "Ⅵ", "Ⅶ", "Ⅷ", "Ⅸ", "Ⅹ", "Ⅺ", "Ⅻ", "Ⅼ", "Ⅽ", "Ⅾ", "Ⅿ",
            // Numerais romanos (minúsculos)
            "ⅰ", "ⅱ", "ⅲ", "ⅳ", "ⅴ", "ⅵ", "ⅶ", "ⅷ", "ⅸ", "ⅹ", "ⅺ", "ⅻ", "ⅼ", "ⅽ", "ⅾ", "ⅿ",
            // Números circulados
            "⓪", "①", "②", "③", "④", "⑤", "⑥", "⑦", "⑧", "⑨", "⑩", "⑪", "⑫", "⑬", "⑭", "⑮", "⑯",
            "⑰", "⑱", "⑲", "⑳", // Números em disco
            "❶", "❷", "❸", "❹", "❺", "❻", "❼", "❽", "❾", "❿", // Sobrescritos
            "⁰", "¹", "²", "³", "⁴", "⁵", "⁶", "⁷", "⁸", "⁹", "⁺", "⁻", "⁼", "⁽", "⁾", "ⁿ",
            // Subscritos
            "₀", "₁", "₂", "₃", "₄", "₅", "₆", "₇", "₈", "₉", "₊", "₋", "₌", "₍", "₎",
        ],
    },
    SymbolGroup {
        name: "Pontuação e Tipografia",
        symbols: &[
            "\u{201C}", "\u{201D}", "\u{2018}", "\u{2019}", "«", "»", "‹", "›", "„", "‚", "–", "—",
            "―", "…", "‰", "‱", "¡", "¿", "‽", "⁂", "•", "‣", "◦", "▪", "▫", "¶", "§", "※", "№",
            "℗", "′", "″", "‾",
        ],
    },
    SymbolGroup {
        name: "Moeda",
        symbols: &[
            "$", "€", "£", "¥", "¢", "₹", "₩", "₽", "₺", "₴", "₦", "₫", "₪", "₡", "₱", "₲", "₵",
            "₸", "₮", "฿", "৳", "₭", "¤",
        ],
    },
    SymbolGroup {
        name: "Letras Gregas",
        symbols: &[
            // Minúsculas
            "α", "β", "γ", "δ", "ε", "ζ", "η", "θ", "ι", "κ", "λ", "μ", "ν", "ξ", "ο", "π", "ρ",
            "σ", "ς", "τ", "υ", "φ", "χ", "ψ", "ω", // Maiúsculas
            "Α", "Β", "Γ", "Δ", "Ε", "Ζ", "Η", "Θ", "Ι", "Κ", "Λ", "Μ", "Ν", "Ξ", "Ο", "Π", "Ρ",
            "Σ", "Τ", "Υ", "Φ", "Χ", "Ψ", "Ω",
        ],
    },
    SymbolGroup {
        name: "Técnicos e Legais",
        symbols: &[
            "©", "®", "™", "℠", "†", "‡", "⌘", "⌥", "⌦", "⌫", "⏎", "⎋", "✓", "✔", "✗", "✘", "☑",
            "☐", "☒", "⚠",
        ],
    },
    SymbolGroup {
        name: "Jogos",
        symbols: &[
            // Naipes
            "♠", "♣", "♥", "♦", "♤", "♧", "♡", "♢", // Xadrez
            "♔", "♕", "♖", "♗", "♘", "♙", "♚", "♛", "♜", "♝", "♞", "♟", // Dados
            "⚀", "⚁", "⚂", "⚃", "⚄", "⚅",
        ],
    },
    SymbolGroup {
        name: "Clima e Natureza",
        symbols: &[
            "☀", "☁", "☂", "☃", "☄", "☽", "☾", "⛅", "⛈", "⛄", "❄", "❅", "❆", "☔", "☘", "❀", "✿",
        ],
    },
    SymbolGroup {
        name: "Música",
        symbols: &["♩", "♪", "♫", "♬", "♭", "♮", "♯", "𝄞"],
    },
];

pub const EMOJI_CATALOG: &[SymbolGroup] = &[
    SymbolGroup {
        name: "Emojis - Smileys e Emoções",
        symbols: &[
            "😀", "😃", "😄", "😁", "😆", "😅", "🤣", "😂", "🙂", "😉", "😊", "😇", "🥰", "😍",
            "😘", "😋", "😛", "🤪", "🤑", "🤗", "🤔", "😐", "😴", "😷", "🤒", "🥵", "🥶", "🤯",
            "🥳", "😎", "🤓", "😢", "😭", "😡", "🤬", "😱", "🥺", "😬", "🙄", "😏",
        ],
    },
    SymbolGroup {
        name: "Emojis - Pessoas e Gestos",
        symbols: &[
            "👋", "🤚", "✋", "🖖", "👌", "✌", "🤞", "🤟", "🤘", "🤙", "👈", "👉", "👆", "👇",
            "👍", "👎", "✊", "👊", "👏", "🙌", "🙏", "💪", "👀", "👶", "🧑", "👨", "👩", "👴",
            "👵", "🙋", "🙅", "🙆", "💁", "🤷", "🤦", "👮", "🕵", "👷", "🤴", "👸", "🎅",
        ],
    },
    SymbolGroup {
        name: "Emojis - Animais e Natureza",
        symbols: &[
            "🐶", "🐱", "🐭", "🐹", "🐰", "🦊", "🐻", "🐼", "🐨", "🐯", "🦁", "🐮", "🐷", "🐸",
            "🐵", "🐔", "🐧", "🐦", "🦆", "🦉", "🐺", "🐴", "🦄", "🐝", "🦋", "🐢", "🐍", "🐙",
            "🐬", "🐳", "🌵", "🌲", "🌳", "🌴", "🌸", "🌻", "🌹", "🍀", "🍁", "🌙", "⭐", "⚡",
            "🔥", "🌈",
        ],
    },
    SymbolGroup {
        name: "Emojis - Comida e Bebida",
        symbols: &[
            "🍎", "🍌", "🍇", "🍓", "🍉", "🍒", "🍑", "🥑", "🍅", "🥕", "🌽", "🍞", "🧀", "🥚",
            "🍳", "🥓", "🍔", "🍟", "🍕", "🌭", "🌮", "🍣", "🍜", "🍦", "🍩", "🎂", "🍰", "🍫",
            "🍭", "🍿", "☕", "🍵", "🥤", "🍺", "🍷", "🍾",
        ],
    },
    SymbolGroup {
        name: "Emojis - Viagem e Lugares",
        symbols: &[
            "🚗", "🚕", "🚌", "🚑", "🚒", "🚲", "🚀", "✈", "🚁", "⛵", "🚢", "🚦", "🗺", "🗽", "🏰",
            "🎡", "🏖", "🏔", "⛺", "🏠", "🏢", "🏥", "🏦", "🏫", "⛪",
        ],
    },
    SymbolGroup {
        name: "Emojis - Atividades",
        symbols: &[
            "⚽", "🏀", "🏈", "⚾", "🎾", "🏐", "🎱", "🏓", "🏸", "🎣", "🎿", "🏋", "🏊", "🚴",
            "🏆", "🥇", "🎮", "🎲", "🎯", "🎨", "🎬", "🎤", "🎧", "🎸", "🎉",
        ],
    },
    SymbolGroup {
        name: "Emojis - Objetos",
        symbols: &[
            "⌚", "📱", "💻", "⌨", "🖥", "📷", "🎥", "📺", "🔋", "💡", "🔦", "📡", "💰", "💳", "💎",
            "🔧", "🔨", "⚙", "🔑", "🚪", "🛏", "🎁", "🎈", "✉", "📦", "📅", "📚", "📖", "🔗", "📌",
            "✂", "✏", "🔍", "🔒", "🔓",
        ],
    },
    SymbolGroup {
        name: "Emojis - Símbolos",
        symbols: &[
            "❤", "🧡", "💛", "💚", "💙", "💜", "🖤", "🤍", "🤎", "💔", "❣", "💕", "💞", "💓", "💗",
            "💖", "💘", "💝", "✅", "❌", "❗", "❓", "‼", "⁉", "💯", "🔴", "🟠", "🟡", "🟢", "🔵",
            "🟣", "⚪", "⚫", "🔶", "🔷", "🔺", "🔻",
        ],
    },
    SymbolGroup {
        name: "Emojis - Bandeiras",
        symbols: &[
            "🏳",
            "🏴",
            "🏁",
            "🚩",
            "🏳\u{200D}🌈",
            "🇧🇷",
            "🇺🇸",
            "🇵🇹",
            "🇪🇸",
            "🇫🇷",
            "🇩🇪",
            "🇮🇹",
            "🇬🇧",
            "🇯🇵",
            "🇨🇳",
            "🇦🇷",
            "🇨🇦",
            "🇦🇺",
            "🇲🇽",
            "🇮🇳",
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_not_empty_and_groups_have_symbols() {
        assert!(!CATALOG.is_empty());
        for group in CATALOG {
            assert!(!group.name.is_empty());
            assert!(!group.symbols.is_empty());
        }
    }

    #[test]
    fn emoji_catalog_is_not_empty_and_groups_have_symbols() {
        assert!(!EMOJI_CATALOG.is_empty());
        for group in EMOJI_CATALOG {
            assert!(!group.name.is_empty());
            assert!(!group.symbols.is_empty());
        }
    }
}
