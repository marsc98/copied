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
}
