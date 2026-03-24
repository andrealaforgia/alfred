//! Language registry: maps file extensions to tree-sitter grammars and queries.

use tree_sitter::Language;

/// Configuration for a single supported language.
pub struct LanguageConfig {
    pub id: &'static str,
    pub extensions: &'static [&'static str],
    pub grammar: Language,
    pub highlight_query: &'static str,
}

static RUST_HIGHLIGHTS: &str = include_str!("../queries/rust/highlights.scm");
static PYTHON_HIGHLIGHTS: &str = include_str!("../queries/python/highlights.scm");
static JAVASCRIPT_HIGHLIGHTS: &str = include_str!("../queries/javascript/highlights.scm");

fn rust_config() -> LanguageConfig {
    LanguageConfig {
        id: "rust",
        extensions: &[".rs"],
        grammar: tree_sitter_rust::LANGUAGE.into(),
        highlight_query: RUST_HIGHLIGHTS,
    }
}

fn python_config() -> LanguageConfig {
    LanguageConfig {
        id: "python",
        extensions: &[".py", ".pyi"],
        grammar: tree_sitter_python::LANGUAGE.into(),
        highlight_query: PYTHON_HIGHLIGHTS,
    }
}

fn javascript_config() -> LanguageConfig {
    LanguageConfig {
        id: "javascript",
        extensions: &[".js", ".mjs", ".cjs", ".jsx"],
        grammar: tree_sitter_javascript::LANGUAGE.into(),
        highlight_query: JAVASCRIPT_HIGHLIGHTS,
    }
}

/// All available language configs.
pub fn all_languages() -> Vec<LanguageConfig> {
    vec![rust_config(), python_config(), javascript_config()]
}

/// Detects language from a filename by matching the file extension.
///
/// Returns the language id (e.g., "rust") if recognized, None otherwise.
pub fn detect_language(filename: &str) -> Option<&'static str> {
    let languages = all_languages();
    for lang in &languages {
        for ext in lang.extensions {
            if filename.ends_with(ext) {
                return Some(lang.id);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_rs_extension_when_detect_language_then_returns_rust() {
        assert_eq!(detect_language("main.rs"), Some("rust"));
    }

    #[test]
    fn given_lib_rs_when_detect_language_then_returns_rust() {
        assert_eq!(detect_language("lib.rs"), Some("rust"));
    }

    #[test]
    fn given_py_extension_when_detect_language_then_returns_python() {
        assert_eq!(detect_language("app.py"), Some("python"));
    }

    #[test]
    fn given_pyi_extension_when_detect_language_then_returns_python() {
        assert_eq!(detect_language("types.pyi"), Some("python"));
    }

    #[test]
    fn given_js_extension_when_detect_language_then_returns_javascript() {
        assert_eq!(detect_language("index.js"), Some("javascript"));
    }

    #[test]
    fn given_mjs_extension_when_detect_language_then_returns_javascript() {
        assert_eq!(detect_language("module.mjs"), Some("javascript"));
    }

    #[test]
    fn given_jsx_extension_when_detect_language_then_returns_javascript() {
        assert_eq!(detect_language("component.jsx"), Some("javascript"));
    }

    #[test]
    fn given_txt_extension_when_detect_language_then_returns_none() {
        assert_eq!(detect_language("readme.txt"), None);
    }

    #[test]
    fn given_no_extension_when_detect_language_then_returns_none() {
        assert_eq!(detect_language("Makefile"), None);
    }

    #[test]
    fn given_empty_filename_when_detect_language_then_returns_none() {
        assert_eq!(detect_language(""), None);
    }

    #[test]
    fn rust_config_has_valid_grammar() {
        let config = rust_config();
        assert_eq!(config.id, "rust");
        assert!(config.extensions.contains(&".rs"));
        assert!(!config.highlight_query.is_empty());
    }

    #[test]
    fn python_config_has_valid_grammar() {
        let config = python_config();
        assert_eq!(config.id, "python");
        assert!(config.extensions.contains(&".py"));
        assert!(!config.highlight_query.is_empty());
    }

    #[test]
    fn javascript_config_has_valid_grammar() {
        let config = javascript_config();
        assert_eq!(config.id, "javascript");
        assert!(config.extensions.contains(&".js"));
        assert!(!config.highlight_query.is_empty());
    }
}
