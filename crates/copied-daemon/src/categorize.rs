use copied_core::Category;

pub fn detect(text: &str) -> Category {
    let trimmed = text.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Category::Url;
    }
    if looks_like_code(trimmed) {
        return Category::Codigo;
    }
    Category::Outro
}

fn looks_like_code(text: &str) -> bool {
    text.contains("fn ") || text.contains("def ") || text.contains('{') || text.contains(';')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_url() {
        assert_eq!(detect("https://example.com/path"), Category::Url);
        assert_eq!(detect("http://example.com"), Category::Url);
    }

    #[test]
    fn detect_code_like_text() {
        assert_eq!(detect("fn main() { println!(\"hi\"); }"), Category::Codigo);
        assert_eq!(detect("def foo():\n    pass"), Category::Codigo);
    }

    #[test]
    fn detect_falls_back_to_outro() {
        assert_eq!(detect("apenas um texto qualquer"), Category::Outro);
        assert_eq!(detect(""), Category::Outro);
    }
}
