use gpui::SharedString;

use crate::highlighter::LanguageConfig;

#[cfg(not(feature = "tree-sitter-languages"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, enum_iterator::Sequence)]
pub enum Language {
    Json,
}

#[cfg(feature = "tree-sitter-languages")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, enum_iterator::Sequence)]
pub enum Language {
    Json,
    Plain,
    Bash,
    C,
    CMake,
    CSharp,
    Cpp,
    Css,
    Diff,
    Ejs,
    Elixir,
    Erb,
    Go,
    GraphQL,
    Html,
    Java,
    JavaScript,
    JsDoc,
    Make,
    Markdown,
    MarkdownInline,
    Php,
    Proto,
    Python,
    Ruby,
    Rust,
    Scala,
    Sql,
    Swift,
    Toml,
    Tsx,
    TypeScript,
    Yaml,
    Zig,
}

impl From<Language> for SharedString {
    fn from(language: Language) -> Self {
        language.name().into()
    }
}

impl Language {
    pub fn all() -> impl Iterator<Item = Self> {
        enum_iterator::all::<Language>()
    }

    pub fn name(&self) -> &'static str {
        #[cfg(not(feature = "tree-sitter-languages"))]
        return "json";

        #[cfg(feature = "tree-sitter-languages")]
        match self {
            Self::Plain => "text",
            Self::Bash => "bash",
            Self::C => "c",
            Self::CMake => "cmake",
            Self::CSharp => "csharp",
            Self::Cpp => "cpp",
            Self::Css => "css",
            Self::Diff => "diff",
            Self::Ejs => "ejs",
            Self::Elixir => "elixir",
            Self::Erb => "erb",
            Self::Go => "go",
            Self::GraphQL => "graphql",
            Self::Html => "html",
            Self::Java => "java",
            Self::JavaScript => "javascript",
            Self::JsDoc => "jsdoc",
            Self::Json => "json",
            Self::Make => "make",
            Self::Markdown => "markdown",
            Self::MarkdownInline => "markdown_inline",
            Self::Php => "php",
            Self::Proto => "proto",
            Self::Python => "python",
            Self::Ruby => "ruby",
            Self::Rust => "rust",
            Self::Scala => "scala",
            Self::Sql => "sql",
            Self::Swift => "swift",
            Self::Toml => "toml",
            Self::Tsx => "tsx",
            Self::TypeScript => "typescript",
            Self::Yaml => "yaml",
            Self::Zig => "zig",
        }
    }

    #[allow(unused)]
    pub fn from_str(s: &str) -> Self {
        #[cfg(not(feature = "tree-sitter-languages"))]
        return Self::Json;

        #[cfg(feature = "tree-sitter-languages")]
        match s {
            "bash" | "sh" => Self::Bash,
            "c" => Self::C,
            "cmake" => Self::CMake,
            "cpp" | "c++" => Self::Cpp,
            "csharp" | "cs" => Self::CSharp,
            "css" | "scss" => Self::Css,
            "diff" => Self::Diff,
            "ejs" => Self::Ejs,
            "elixir" | "ex" => Self::Elixir,
            "erb" => Self::Erb,
            "go" => Self::Go,
            "graphql" => Self::GraphQL,
            "html" => Self::Html,
            "java" => Self::Java,
            "javascript" | "js" => Self::JavaScript,
            "jsdoc" => Self::JsDoc,
            "json" | "jsonc" => Self::Json,
            "make" | "makefile" => Self::Make,
            "markdown" | "md" | "mdx" => Self::Markdown,
            "markdown_inline" | "markdown-inline" => Self::MarkdownInline,
            "php" | "php3" | "php4" | "php5" | "phtml" => Self::Php,
            "proto" | "protobuf" => Self::Proto,
            "python" | "py" => Self::Python,
            "ruby" | "rb" => Self::Ruby,
            "rust" | "rs" => Self::Rust,
            "scala" => Self::Scala,
            "sql" => Self::Sql,
            "swift" => Self::Swift,
            "toml" => Self::Toml,
            "tsx" => Self::Tsx,
            "typescript" | "ts" => Self::TypeScript,
            "yaml" | "yml" => Self::Yaml,
            "zig" => Self::Zig,
            _ => Self::Plain,
        }
    }

    #[allow(unused)]
    pub(super) fn injection_languages(&self) -> Vec<SharedString> {
        #[cfg(not(feature = "tree-sitter-languages"))]
        return vec![];

        #[cfg(feature = "tree-sitter-languages")]
        match self {
            Self::Markdown => vec!["markdown-inline", "html", "toml", "yaml"],
            Self::MarkdownInline => vec![],
            Self::Html => vec!["javascript", "css"],
            Self::Rust => vec!["rust"],
            Self::JavaScript | Self::TypeScript => vec![
                "jsdoc",
                "json",
                "css",
                "html",
                "sql",
                "typescript",
                "javascript",
                "tsx",
                "yaml",
                "graphql",
            ],
            Self::Php => vec![
                "php",
                "html",
                "css",
                "javascript",
                "json",
                "jsdoc",
                "graphql",
            ],
            _ => vec![],
        }
        .into_iter()
        .map(|s| s.into())
        .collect()
    }

    /// Return the language info for the language.
    ///
    /// (language, query, injection, locals)
    pub(super) fn config(&self) -> LanguageConfig {
        #[cfg(not(feature = "tree-sitter-languages"))]
        let (language, query, injection, locals) = match self {
            Self::Json => (
                tree_sitter_json::LANGUAGE,
                include_str!("languages/json/highlights.scm"),
                "",
                "",
            ),
        };

        #[cfg(feature = "tree-sitter-languages")]
        let (language, query, injection, locals) = match self {
            Self::Plain => (tree_sitter_json::LANGUAGE, "", "", ""),
            Self::Json => (
                tree_sitter_json::LANGUAGE,
                include_str!("languages/json/highlights.scm"),
                "",
                "",
            ),
            Self::Markdown => (
                tree_sitter_md::LANGUAGE,
                include_str!("languages/markdown/highlights.scm"),
                include_str!("languages/markdown/injections.scm"),
                "",
            ),
            Self::MarkdownInline => (
                tree_sitter_md::INLINE_LANGUAGE,
                include_str!("languages/markdown_inline/highlights.scm"),
                "",
                "",
            ),
            Self::Toml => (
                tree_sitter_toml_ng::LANGUAGE,
                tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            Self::Yaml => (
                tree_sitter_yaml::LANGUAGE,
                tree_sitter_yaml::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            Self::Rust => (
                tree_sitter_rust::LANGUAGE,
                include_str!("languages/rust/highlights.scm"),
                include_str!("languages/rust/injections.scm"),
                "",
            ),
            Self::Go => (
                tree_sitter_go::LANGUAGE,
                include_str!("languages/go/highlights.scm"),
                "",
                "",
            ),
            Self::C => (
                tree_sitter_c::LANGUAGE,
                tree_sitter_c::HIGHLIGHT_QUERY,
                "",
                "",
            ),
            Self::Cpp => (
                tree_sitter_cpp::LANGUAGE,
                tree_sitter_cpp::HIGHLIGHT_QUERY,
                "",
                "",
            ),
            Self::JavaScript => (
                tree_sitter_javascript::LANGUAGE,
                include_str!("languages/javascript/highlights.scm"),
                include_str!("languages/javascript/injections.scm"),
                tree_sitter_javascript::LOCALS_QUERY,
            ),
            Self::JsDoc => (
                tree_sitter_jsdoc::LANGUAGE,
                tree_sitter_jsdoc::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            Self::Zig => (
                tree_sitter_zig::LANGUAGE,
                include_str!("languages/zig/highlights.scm"),
                include_str!("languages/zig/injections.scm"),
                "",
            ),
            Self::Java => (
                tree_sitter_java::LANGUAGE,
                tree_sitter_java::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            Self::Python => (
                tree_sitter_python::LANGUAGE,
                tree_sitter_python::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            Self::Ruby => (
                tree_sitter_ruby::LANGUAGE,
                tree_sitter_ruby::HIGHLIGHTS_QUERY,
                "",
                tree_sitter_ruby::LOCALS_QUERY,
            ),
            Self::Bash => (
                tree_sitter_bash::LANGUAGE,
                tree_sitter_bash::HIGHLIGHT_QUERY,
                "",
                "",
            ),
            Self::Html => (
                tree_sitter_html::LANGUAGE,
                include_str!("languages/html/highlights.scm"),
                include_str!("languages/html/injections.scm"),
                "",
            ),
            Self::Css => (
                tree_sitter_css::LANGUAGE,
                tree_sitter_css::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            Self::Swift => (tree_sitter_swift::LANGUAGE, "", "", ""),
            Self::Scala => (
                tree_sitter_scala::LANGUAGE,
                tree_sitter_scala::HIGHLIGHTS_QUERY,
                "",
                tree_sitter_scala::LOCALS_QUERY,
            ),
            Self::Sql => (
                tree_sitter_sequel::LANGUAGE,
                tree_sitter_sequel::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            Self::CSharp => (tree_sitter_c_sharp::LANGUAGE, "", "", ""),
            Self::GraphQL => (tree_sitter_graphql::LANGUAGE, "", "", ""),
            Self::Proto => (tree_sitter_proto::LANGUAGE, "", "", ""),
            Self::Make => (
                tree_sitter_make::LANGUAGE,
                tree_sitter_make::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            Self::CMake => (tree_sitter_cmake::LANGUAGE, "", "", ""),
            Self::TypeScript => (
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
                include_str!("languages/typescript/highlights.scm"),
                include_str!("languages/javascript/injections.scm"),
                tree_sitter_typescript::LOCALS_QUERY,
            ),
            Self::Tsx => (
                tree_sitter_typescript::LANGUAGE_TSX,
                tree_sitter_typescript::HIGHLIGHTS_QUERY,
                "",
                tree_sitter_typescript::LOCALS_QUERY,
            ),
            Self::Diff => (
                tree_sitter_diff::LANGUAGE,
                tree_sitter_diff::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            Self::Elixir => (
                tree_sitter_elixir::LANGUAGE,
                tree_sitter_elixir::HIGHLIGHTS_QUERY,
                tree_sitter_elixir::INJECTIONS_QUERY,
                "",
            ),
            Self::Erb => (
                tree_sitter_embedded_template::LANGUAGE,
                tree_sitter_embedded_template::HIGHLIGHTS_QUERY,
                tree_sitter_embedded_template::INJECTIONS_EJS_QUERY,
                "",
            ),
            Self::Ejs => (
                tree_sitter_embedded_template::LANGUAGE,
                tree_sitter_embedded_template::HIGHLIGHTS_QUERY,
                tree_sitter_embedded_template::INJECTIONS_EJS_QUERY,
                "",
            ),
            Self::Php => (
                tree_sitter_php::LANGUAGE_PHP,
                tree_sitter_php::HIGHLIGHTS_QUERY,
                include_str!("languages/php/injections.scm"),
                "",
            ),
        };

        let language = tree_sitter::Language::new(language);

        LanguageConfig::new(
            self.name(),
            language,
            self.injection_languages(),
            query,
            injection,
            locals,
        )
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "tree-sitter-languages")]
    fn test_language_name() {
        use super::*;

        assert_eq!(Language::MarkdownInline.name(), "markdown_inline");
        assert_eq!(Language::Markdown.name(), "markdown");
        assert_eq!(Language::Json.name(), "json");
        assert_eq!(Language::Yaml.name(), "yaml");
        assert_eq!(Language::Rust.name(), "rust");
        assert_eq!(Language::Go.name(), "go");
        assert_eq!(Language::C.name(), "c");
        assert_eq!(Language::Cpp.name(), "cpp");
        assert_eq!(Language::Sql.name(), "sql");
        assert_eq!(Language::JavaScript.name(), "javascript");
        assert_eq!(Language::Zig.name(), "zig");
        assert_eq!(Language::CSharp.name(), "csharp");
        assert_eq!(Language::TypeScript.name(), "typescript");
        assert_eq!(Language::Tsx.name(), "tsx");
        assert_eq!(Language::Diff.name(), "diff");
        assert_eq!(Language::Elixir.name(), "elixir");
        assert_eq!(Language::Erb.name(), "erb");
        assert_eq!(Language::Ejs.name(), "ejs");
    }
}
