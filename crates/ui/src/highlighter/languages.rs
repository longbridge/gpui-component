use gpui::SharedString;

use crate::highlighter::LanguageConfig;

#[cfg(not(any(
    feature = "tree-sitter-languages",
    feature = "tree-sitter-essential"
)))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, enum_iterator::Sequence)]
pub enum Language {
    Json,
}

#[cfg(any(
    feature = "tree-sitter-languages",
    feature = "tree-sitter-essential"
))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, enum_iterator::Sequence)]
pub enum Language {
    Json,
    Plain,
    #[cfg(feature = "tree-sitter-bash")]
    Bash,
    #[cfg(feature = "tree-sitter-c")]
    C,
    #[cfg(feature = "tree-sitter-cmake")]
    CMake,
    #[cfg(feature = "tree-sitter-c-sharp")]
    CSharp,
    #[cfg(feature = "tree-sitter-cpp")]
    Cpp,
    #[cfg(feature = "tree-sitter-css")]
    Css,
    #[cfg(feature = "tree-sitter-diff")]
    Diff,
    #[cfg(feature = "tree-sitter-embedded-template")]
    Ejs,
    #[cfg(feature = "tree-sitter-elixir")]
    Elixir,
    #[cfg(feature = "tree-sitter-embedded-template")]
    Erb,
    #[cfg(feature = "tree-sitter-go")]
    Go,
    #[cfg(feature = "tree-sitter-graphql")]
    GraphQL,
    #[cfg(feature = "tree-sitter-html")]
    Html,
    #[cfg(feature = "tree-sitter-java")]
    Java,
    #[cfg(feature = "tree-sitter-javascript")]
    JavaScript,
    #[cfg(feature = "tree-sitter-jsdoc")]
    JsDoc,
    #[cfg(feature = "tree-sitter-kotlin-sg")]
    Kotlin,
    #[cfg(feature = "tree-sitter-make")]
    Make,
    #[cfg(feature = "tree-sitter-md")]
    Markdown,
    #[cfg(feature = "tree-sitter-md")]
    MarkdownInline,
    #[cfg(feature = "tree-sitter-php")]
    Php,
    #[cfg(feature = "tree-sitter-proto")]
    Proto,
    #[cfg(feature = "tree-sitter-python")]
    Python,
    #[cfg(feature = "tree-sitter-ruby")]
    Ruby,
    #[cfg(feature = "tree-sitter-rust")]
    Rust,
    #[cfg(feature = "tree-sitter-scala")]
    Scala,
    #[cfg(feature = "tree-sitter-sequel")]
    Sql,
    #[cfg(feature = "tree-sitter-svelte-next")]
    Svelte,
    #[cfg(feature = "tree-sitter-swift")]
    Swift,
    #[cfg(feature = "tree-sitter-toml-ng")]
    Toml,
    #[cfg(feature = "tree-sitter-typescript")]
    Tsx,
    #[cfg(feature = "tree-sitter-typescript")]
    TypeScript,
    #[cfg(feature = "tree-sitter-yaml")]
    Yaml,
    #[cfg(feature = "tree-sitter-zig")]
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
        #[cfg(not(any(
            feature = "tree-sitter-languages",
            feature = "tree-sitter-essential"
        )))]
        return "json";

        #[cfg(any(
            feature = "tree-sitter-languages",
            feature = "tree-sitter-essential"
        ))]
        match self {
            Self::Plain => "text",
            Self::Json => "json",
            #[cfg(feature = "tree-sitter-bash")]
            Self::Bash => "bash",
            #[cfg(feature = "tree-sitter-c")]
            Self::C => "c",
            #[cfg(feature = "tree-sitter-cmake")]
            Self::CMake => "cmake",
            #[cfg(feature = "tree-sitter-c-sharp")]
            Self::CSharp => "csharp",
            #[cfg(feature = "tree-sitter-cpp")]
            Self::Cpp => "cpp",
            #[cfg(feature = "tree-sitter-css")]
            Self::Css => "css",
            #[cfg(feature = "tree-sitter-diff")]
            Self::Diff => "diff",
            #[cfg(feature = "tree-sitter-embedded-template")]
            Self::Ejs => "ejs",
            #[cfg(feature = "tree-sitter-elixir")]
            Self::Elixir => "elixir",
            #[cfg(feature = "tree-sitter-embedded-template")]
            Self::Erb => "erb",
            #[cfg(feature = "tree-sitter-go")]
            Self::Go => "go",
            #[cfg(feature = "tree-sitter-graphql")]
            Self::GraphQL => "graphql",
            #[cfg(feature = "tree-sitter-html")]
            Self::Html => "html",
            #[cfg(feature = "tree-sitter-java")]
            Self::Java => "java",
            #[cfg(feature = "tree-sitter-javascript")]
            Self::JavaScript => "javascript",
            #[cfg(feature = "tree-sitter-jsdoc")]
            Self::JsDoc => "jsdoc",
            #[cfg(feature = "tree-sitter-kotlin-sg")]
            Self::Kotlin => "kotlin",
            #[cfg(feature = "tree-sitter-make")]
            Self::Make => "make",
            #[cfg(feature = "tree-sitter-md")]
            Self::Markdown => "markdown",
            #[cfg(feature = "tree-sitter-md")]
            Self::MarkdownInline => "markdown_inline",
            #[cfg(feature = "tree-sitter-php")]
            Self::Php => "php",
            #[cfg(feature = "tree-sitter-proto")]
            Self::Proto => "proto",
            #[cfg(feature = "tree-sitter-python")]
            Self::Python => "python",
            #[cfg(feature = "tree-sitter-ruby")]
            Self::Ruby => "ruby",
            #[cfg(feature = "tree-sitter-rust")]
            Self::Rust => "rust",
            #[cfg(feature = "tree-sitter-scala")]
            Self::Scala => "scala",
            #[cfg(feature = "tree-sitter-sequel")]
            Self::Sql => "sql",
            #[cfg(feature = "tree-sitter-svelte-next")]
            Self::Svelte => "svelte",
            #[cfg(feature = "tree-sitter-swift")]
            Self::Swift => "swift",
            #[cfg(feature = "tree-sitter-toml-ng")]
            Self::Toml => "toml",
            #[cfg(feature = "tree-sitter-typescript")]
            Self::Tsx => "tsx",
            #[cfg(feature = "tree-sitter-typescript")]
            Self::TypeScript => "typescript",
            #[cfg(feature = "tree-sitter-yaml")]
            Self::Yaml => "yaml",
            #[cfg(feature = "tree-sitter-zig")]
            Self::Zig => "zig",
        }
    }

    #[allow(unused)]
    pub fn from_str(s: &str) -> Self {
        #[cfg(not(any(
            feature = "tree-sitter-languages",
            feature = "tree-sitter-essential"
        )))]
        return Self::Json;

        #[cfg(any(
            feature = "tree-sitter-languages",
            feature = "tree-sitter-essential"
        ))]
        match s {
            #[cfg(feature = "tree-sitter-bash")]
            "bash" | "sh" => Self::Bash,
            #[cfg(feature = "tree-sitter-c")]
            "c" => Self::C,
            #[cfg(feature = "tree-sitter-cmake")]
            "cmake" => Self::CMake,
            #[cfg(feature = "tree-sitter-cpp")]
            "cpp" | "c++" => Self::Cpp,
            #[cfg(feature = "tree-sitter-c-sharp")]
            "csharp" | "cs" => Self::CSharp,
            #[cfg(feature = "tree-sitter-css")]
            "css" | "scss" => Self::Css,
            #[cfg(feature = "tree-sitter-diff")]
            "diff" => Self::Diff,
            #[cfg(feature = "tree-sitter-embedded-template")]
            "ejs" => Self::Ejs,
            #[cfg(feature = "tree-sitter-elixir")]
            "elixir" | "ex" => Self::Elixir,
            #[cfg(feature = "tree-sitter-embedded-template")]
            "erb" => Self::Erb,
            #[cfg(feature = "tree-sitter-go")]
            "go" => Self::Go,
            #[cfg(feature = "tree-sitter-graphql")]
            "graphql" => Self::GraphQL,
            #[cfg(feature = "tree-sitter-html")]
            "html" => Self::Html,
            #[cfg(feature = "tree-sitter-java")]
            "java" => Self::Java,
            #[cfg(feature = "tree-sitter-javascript")]
            "javascript" | "js" => Self::JavaScript,
            #[cfg(feature = "tree-sitter-jsdoc")]
            "jsdoc" => Self::JsDoc,
            "json" | "jsonc" => Self::Json,
            #[cfg(feature = "tree-sitter-kotlin-sg")]
            "kt" | "kts" | "ktm" => Self::Kotlin,
            #[cfg(feature = "tree-sitter-make")]
            "make" | "makefile" => Self::Make,
            #[cfg(feature = "tree-sitter-md")]
            "markdown" | "md" | "mdx" => Self::Markdown,
            #[cfg(feature = "tree-sitter-md")]
            "markdown_inline" | "markdown-inline" => Self::MarkdownInline,
            #[cfg(feature = "tree-sitter-php")]
            "php" | "php3" | "php4" | "php5" | "phtml" => Self::Php,
            #[cfg(feature = "tree-sitter-proto")]
            "proto" | "protobuf" => Self::Proto,
            #[cfg(feature = "tree-sitter-python")]
            "python" | "py" => Self::Python,
            #[cfg(feature = "tree-sitter-ruby")]
            "ruby" | "rb" => Self::Ruby,
            #[cfg(feature = "tree-sitter-rust")]
            "rust" | "rs" => Self::Rust,
            #[cfg(feature = "tree-sitter-scala")]
            "scala" => Self::Scala,
            #[cfg(feature = "tree-sitter-sequel")]
            "sql" => Self::Sql,
            #[cfg(feature = "tree-sitter-svelte-next")]
            "svelte" => Self::Svelte,
            #[cfg(feature = "tree-sitter-swift")]
            "swift" => Self::Swift,
            #[cfg(feature = "tree-sitter-toml-ng")]
            "toml" => Self::Toml,
            #[cfg(feature = "tree-sitter-typescript")]
            "tsx" => Self::Tsx,
            #[cfg(feature = "tree-sitter-typescript")]
            "typescript" | "ts" => Self::TypeScript,
            #[cfg(feature = "tree-sitter-yaml")]
            "yaml" | "yml" => Self::Yaml,
            #[cfg(feature = "tree-sitter-zig")]
            "zig" => Self::Zig,
            _ => Self::Plain,
        }
    }

    #[allow(unused)]
    pub(super) fn injection_languages(&self) -> Vec<SharedString> {
        #[cfg(not(any(
            feature = "tree-sitter-languages",
            feature = "tree-sitter-essential"
        )))]
        return vec![];

        #[cfg(any(
            feature = "tree-sitter-languages",
            feature = "tree-sitter-essential"
        ))]
        match self {
            #[cfg(feature = "tree-sitter-md")]
            Self::Markdown => vec!["markdown-inline", "html", "toml", "yaml"],
            #[cfg(feature = "tree-sitter-md")]
            Self::MarkdownInline => vec![],
            #[cfg(feature = "tree-sitter-html")]
            Self::Html => vec!["javascript", "css"],
            #[cfg(feature = "tree-sitter-rust")]
            Self::Rust => vec!["rust"],
            #[cfg(feature = "tree-sitter-javascript")]
            Self::JavaScript => vec![
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
            #[cfg(feature = "tree-sitter-typescript")]
            Self::TypeScript => vec![
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
            #[cfg(feature = "tree-sitter-php")]
            Self::Php => vec![
                "php",
                "html",
                "css",
                "javascript",
                "json",
                "jsdoc",
                "graphql",
            ],
            #[cfg(feature = "tree-sitter-svelte-next")]
            Self::Svelte => vec!["svelte", "html", "css", "typescript"],
            _ => vec![],
        }
        .into_iter()
        .map(|s: &str| s.into())
        .collect()
    }

    /// Return the language info for the language.
    ///
    /// (language, query, injection, locals)
    pub(super) fn config(&self) -> LanguageConfig {
        #[cfg(not(any(
            feature = "tree-sitter-languages",
            feature = "tree-sitter-essential"
        )))]
        let (language, query, injection, locals) = match self {
            Self::Json => (
                tree_sitter_json::LANGUAGE,
                include_str!("languages/json/highlights.scm"),
                "",
                "",
            ),
        };

        #[cfg(any(
            feature = "tree-sitter-languages",
            feature = "tree-sitter-essential"
        ))]
        let (language, query, injection, locals) = match self {
            Self::Plain => (tree_sitter_json::LANGUAGE, "", "", ""),
            Self::Json => (
                tree_sitter_json::LANGUAGE,
                include_str!("languages/json/highlights.scm"),
                "",
                "",
            ),
            #[cfg(feature = "tree-sitter-md")]
            Self::Markdown => (
                tree_sitter_md::LANGUAGE,
                include_str!("languages/markdown/highlights.scm"),
                include_str!("languages/markdown/injections.scm"),
                "",
            ),
            #[cfg(feature = "tree-sitter-md")]
            Self::MarkdownInline => (
                tree_sitter_md::INLINE_LANGUAGE,
                include_str!("languages/markdown_inline/highlights.scm"),
                "",
                "",
            ),
            #[cfg(feature = "tree-sitter-toml-ng")]
            Self::Toml => (
                tree_sitter_toml_ng::LANGUAGE,
                tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            #[cfg(feature = "tree-sitter-yaml")]
            Self::Yaml => (
                tree_sitter_yaml::LANGUAGE,
                tree_sitter_yaml::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            #[cfg(feature = "tree-sitter-rust")]
            Self::Rust => (
                tree_sitter_rust::LANGUAGE,
                include_str!("languages/rust/highlights.scm"),
                include_str!("languages/rust/injections.scm"),
                "",
            ),
            #[cfg(feature = "tree-sitter-go")]
            Self::Go => (
                tree_sitter_go::LANGUAGE,
                include_str!("languages/go/highlights.scm"),
                "",
                "",
            ),
            #[cfg(feature = "tree-sitter-c")]
            Self::C => (
                tree_sitter_c::LANGUAGE,
                tree_sitter_c::HIGHLIGHT_QUERY,
                "",
                "",
            ),
            #[cfg(feature = "tree-sitter-cpp")]
            Self::Cpp => (
                tree_sitter_cpp::LANGUAGE,
                tree_sitter_cpp::HIGHLIGHT_QUERY,
                "",
                "",
            ),
            #[cfg(feature = "tree-sitter-javascript")]
            Self::JavaScript => (
                tree_sitter_javascript::LANGUAGE,
                include_str!("languages/javascript/highlights.scm"),
                include_str!("languages/javascript/injections.scm"),
                tree_sitter_javascript::LOCALS_QUERY,
            ),
            #[cfg(feature = "tree-sitter-jsdoc")]
            Self::JsDoc => (
                tree_sitter_jsdoc::LANGUAGE,
                tree_sitter_jsdoc::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            #[cfg(feature = "tree-sitter-zig")]
            Self::Zig => (
                tree_sitter_zig::LANGUAGE,
                include_str!("languages/zig/highlights.scm"),
                include_str!("languages/zig/injections.scm"),
                "",
            ),
            #[cfg(feature = "tree-sitter-java")]
            Self::Java => (
                tree_sitter_java::LANGUAGE,
                tree_sitter_java::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            #[cfg(feature = "tree-sitter-python")]
            Self::Python => (
                tree_sitter_python::LANGUAGE,
                tree_sitter_python::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            #[cfg(feature = "tree-sitter-ruby")]
            Self::Ruby => (
                tree_sitter_ruby::LANGUAGE,
                tree_sitter_ruby::HIGHLIGHTS_QUERY,
                "",
                tree_sitter_ruby::LOCALS_QUERY,
            ),
            #[cfg(feature = "tree-sitter-bash")]
            Self::Bash => (
                tree_sitter_bash::LANGUAGE,
                tree_sitter_bash::HIGHLIGHT_QUERY,
                "",
                "",
            ),
            #[cfg(feature = "tree-sitter-html")]
            Self::Html => (
                tree_sitter_html::LANGUAGE,
                include_str!("languages/html/highlights.scm"),
                include_str!("languages/html/injections.scm"),
                "",
            ),
            #[cfg(feature = "tree-sitter-css")]
            Self::Css => (
                tree_sitter_css::LANGUAGE,
                tree_sitter_css::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            #[cfg(feature = "tree-sitter-swift")]
            Self::Swift => (tree_sitter_swift::LANGUAGE, "", "", ""),
            #[cfg(feature = "tree-sitter-scala")]
            Self::Scala => (
                tree_sitter_scala::LANGUAGE,
                tree_sitter_scala::HIGHLIGHTS_QUERY,
                "",
                tree_sitter_scala::LOCALS_QUERY,
            ),
            #[cfg(feature = "tree-sitter-sequel")]
            Self::Sql => (
                tree_sitter_sequel::LANGUAGE,
                tree_sitter_sequel::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            #[cfg(feature = "tree-sitter-c-sharp")]
            Self::CSharp => (tree_sitter_c_sharp::LANGUAGE, "", "", ""),
            #[cfg(feature = "tree-sitter-graphql")]
            Self::GraphQL => (tree_sitter_graphql::LANGUAGE, "", "", ""),
            #[cfg(feature = "tree-sitter-proto")]
            Self::Proto => (tree_sitter_proto::LANGUAGE, "", "", ""),
            #[cfg(feature = "tree-sitter-make")]
            Self::Make => (
                tree_sitter_make::LANGUAGE,
                tree_sitter_make::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            #[cfg(feature = "tree-sitter-cmake")]
            Self::CMake => (tree_sitter_cmake::LANGUAGE, "", "", ""),
            #[cfg(feature = "tree-sitter-typescript")]
            Self::TypeScript => (
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
                include_str!("languages/typescript/highlights.scm"),
                include_str!("languages/javascript/injections.scm"),
                tree_sitter_typescript::LOCALS_QUERY,
            ),
            #[cfg(feature = "tree-sitter-typescript")]
            Self::Tsx => (
                tree_sitter_typescript::LANGUAGE_TSX,
                tree_sitter_typescript::HIGHLIGHTS_QUERY,
                "",
                tree_sitter_typescript::LOCALS_QUERY,
            ),
            #[cfg(feature = "tree-sitter-diff")]
            Self::Diff => (
                tree_sitter_diff::LANGUAGE,
                tree_sitter_diff::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            #[cfg(feature = "tree-sitter-elixir")]
            Self::Elixir => (
                tree_sitter_elixir::LANGUAGE,
                tree_sitter_elixir::HIGHLIGHTS_QUERY,
                tree_sitter_elixir::INJECTIONS_QUERY,
                "",
            ),
            #[cfg(feature = "tree-sitter-embedded-template")]
            Self::Erb => (
                tree_sitter_embedded_template::LANGUAGE,
                tree_sitter_embedded_template::HIGHLIGHTS_QUERY,
                tree_sitter_embedded_template::INJECTIONS_EJS_QUERY,
                "",
            ),
            #[cfg(feature = "tree-sitter-embedded-template")]
            Self::Ejs => (
                tree_sitter_embedded_template::LANGUAGE,
                tree_sitter_embedded_template::HIGHLIGHTS_QUERY,
                tree_sitter_embedded_template::INJECTIONS_EJS_QUERY,
                "",
            ),
            #[cfg(feature = "tree-sitter-php")]
            Self::Php => (
                tree_sitter_php::LANGUAGE_PHP,
                tree_sitter_php::HIGHLIGHTS_QUERY,
                include_str!("languages/php/injections.scm"),
                "",
            ),
            #[cfg(feature = "tree-sitter-kotlin-sg")]
            Self::Kotlin => (
                tree_sitter_kotlin_sg::LANGUAGE,
                include_str!("languages/kotlin/highlights.scm"),
                "",
                "",
            ),
            #[cfg(feature = "tree-sitter-svelte-next")]
            Self::Svelte => (
                tree_sitter_svelte_next::LANGUAGE,
                tree_sitter_svelte_next::HIGHLIGHTS_QUERY,
                tree_sitter_svelte_next::INJECTIONS_QUERY,
                tree_sitter_svelte_next::LOCALS_QUERY,
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
