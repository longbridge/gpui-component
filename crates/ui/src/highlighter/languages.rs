pub enum Language {
    Json,
    Markdown,
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
    CSharp,
    GraphQL,
    Proto,
    Make,
    CMake,
}

impl Language {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "json" => Some(Self::Json),
            "markdown" | "md" => Some(Self::Markdown),
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
            "css" => Some(Self::Css),
            "swift" => Some(Self::Swift),
            "scala" => Some(Self::Scala),
            "csharp" | "cs" => Some(Self::CSharp),
            "graphql" => Some(Self::GraphQL),
            "proto" => Some(Self::Proto),
            "make" | "makefile" => Some(Self::Make),
            "cmake" => Some(Self::CMake),
            _ => None,
        }
    }

    pub fn build(&self) -> Option<tree_sitter_highlight::HighlightConfiguration> {
        let (language, query, injection, locals) = match self {
            Self::Json => (
                tree_sitter_json::LANGUAGE,
                tree_sitter_json::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            Self::Markdown => (
                tree_sitter_md::LANGUAGE,
                tree_sitter_md::INJECTION_QUERY_BLOCK,
                tree_sitter_md::INJECTION_QUERY_BLOCK,
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
                tree_sitter_rust::HIGHLIGHTS_QUERY,
                tree_sitter_rust::INJECTIONS_QUERY,
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
                tree_sitter_html::HIGHLIGHTS_QUERY,
                "",
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
        };

        let language = tree_sitter::Language::new(language);
        let name = language.name().unwrap_or("");
        let config = tree_sitter_highlight::HighlightConfiguration::new(
            language, name, query, injection, locals,
        )
        .ok()?;

        Some(config)
    }
}
