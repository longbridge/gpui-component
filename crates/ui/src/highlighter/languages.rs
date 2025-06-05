use gpui::SharedString;
use tree_sitter::Query;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, enum_iterator::Sequence)]
pub enum Language {
    Json,
    Markdown,
    MarkdownInline,
    Toml,
    Yaml,
    Rust,
    Go,
    C,
    Cpp,
    JavaScript,
    Zig,
    Java,
    Python,
    Ruby,
    Bash,
    Html,
    Css,
    Swift,
    Scala,
    Sql,
    CSharp,
    GraphQL,
    Proto,
    Make,
    CMake,
    TypeScript,
    Tsx,
    Diff,
    Elixir,
    Erb,
    Ejs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageConfig {
    pub language: tree_sitter::Language,
    pub highlights: SharedString,
    pub injections: SharedString,
    pub locals: SharedString,
}

impl LanguageConfig {
    pub fn new(
        language: tree_sitter::Language,
        highlights: &str,
        injections: &str,
        locals: &str,
    ) -> Self {
        Self {
            language,
            highlights: SharedString::from(highlights.to_string()),
            injections: SharedString::from(injections.to_string()),
            locals: SharedString::from(locals.to_string()),
        }
    }
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
        match self {
            Self::Json => "json",
            Self::Markdown => "markdown",
            Self::MarkdownInline => "markdown_inline",
            Self::Toml => "toml",
            Self::Yaml => "yaml",
            Self::Rust => "rust",
            Self::Go => "go",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::JavaScript => "javascript",
            Self::Zig => "zig",
            Self::Java => "java",
            Self::Python => "python",
            Self::Ruby => "ruby",
            Self::Bash => "bash",
            Self::Html => "html",
            Self::Css => "css",
            Self::Swift => "swift",
            Self::Scala => "scala",
            Self::Sql => "sql",
            Self::CSharp => "csharp",
            Self::GraphQL => "graphql",
            Self::Proto => "proto",
            Self::Make => "make",
            Self::CMake => "cmake",
            Self::TypeScript => "typescript",
            Self::Tsx => "tsx",
            Self::Diff => "diff",
            Self::Elixir => "elixir",
            Self::Erb => "erb",
            Self::Ejs => "ejs",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "json" | "jsonc" => Some(Self::Json),
            "markdown" | "md" | "mdx" => Some(Self::Markdown),
            "markdown_inline" | "markdown-inline" => Some(Self::MarkdownInline),
            "toml" => Some(Self::Toml),
            "yaml" | "yml" => Some(Self::Yaml),
            "rust" | "rs" => Some(Self::Rust),
            "go" => Some(Self::Go),
            "c" => Some(Self::C),
            "cpp" | "c++" => Some(Self::Cpp),
            "javascript" | "js" => Some(Self::JavaScript),
            "zig" => Some(Self::Zig),
            "java" => Some(Self::Java),
            "python" | "py" => Some(Self::Python),
            "ruby" | "rb" => Some(Self::Ruby),
            "bash" | "sh" => Some(Self::Bash),
            "html" => Some(Self::Html),
            "css" | "scss" => Some(Self::Css),
            "swift" => Some(Self::Swift),
            "scala" => Some(Self::Scala),
            "sql" => Some(Self::Sql),
            "csharp" | "cs" => Some(Self::CSharp),
            "graphql" => Some(Self::GraphQL),
            "proto" | "protobuf" => Some(Self::Proto),
            "make" | "makefile" => Some(Self::Make),
            "cmake" => Some(Self::CMake),
            "typescript" | "ts" => Some(Self::TypeScript),
            "tsx" => Some(Self::Tsx),
            "diff" => Some(Self::Diff),
            "elixir" | "ex" => Some(Self::Elixir),
            "erb" => Some(Self::Erb),
            "ejs" => Some(Self::Ejs),
            _ => None,
        }
    }

    #[allow(unused)]
    pub(super) fn injection_languages(&self) -> Vec<Self> {
        match self {
            Self::Markdown => vec![Self::MarkdownInline, Self::Html, Self::Toml, Self::Yaml],
            Self::MarkdownInline => vec![],
            Self::Html => vec![Self::JavaScript, Self::Css],
            Self::Rust => vec![Self::Rust],
            _ => vec![],
        }
    }

    pub(super) fn query(&self) -> Query {
        let config = self.config();
        Query::new(&config.language, &config.highlights).unwrap()
    }

    /// Return the language info for the language.
    ///
    /// (language, query, injection, locals)
    pub(super) fn config(&self) -> LanguageConfig {
        let (language, query, injection, locals) = match self {
            Self::Json => (
                tree_sitter_json::LANGUAGE,
                tree_sitter_json::HIGHLIGHTS_QUERY,
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
                tree_sitter_go::HIGHLIGHTS_QUERY,
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
                tree_sitter_javascript::HIGHLIGHT_QUERY,
                tree_sitter_javascript::INJECTIONS_QUERY,
                tree_sitter_javascript::LOCALS_QUERY,
            ),
            Self::Zig => (
                tree_sitter_zig::LANGUAGE,
                tree_sitter_zig::HIGHLIGHTS_QUERY,
                tree_sitter_zig::INJECTIONS_QUERY,
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
                tree_sitter_typescript::HIGHLIGHTS_QUERY,
                "",
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
        };

        let language = tree_sitter::Language::new(language);

        LanguageConfig::new(language, query, injection, locals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_name() {
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
