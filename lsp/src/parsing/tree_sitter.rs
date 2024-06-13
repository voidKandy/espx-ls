use tree_sitter::{self, Language};

pub enum ParsableLanguage {
    Rust(Language),
    Go(Language),
    Java(Language),
    C(Language),
    Html(Language),
    Css(Language),
    Lua(Language),
    Javascript(Language),
    None,
    // Strange issue where another version of treesitter is installed  when these are present
    // Markdown(Language),
    // Typescript(Language),
}

const RUST_EXT: &str = "rs";
const GO_EXT: &str = "go";
const JAVA_EXT: &str = "java";
const C_EXT: &str = "c";
const MARKDOWN_EXT: &str = "md";
const HTML_EXT: &str = "html";
const CSS_EXT: &str = "css";
const LUA_EXT: &str = "lua";
const JAVASCRIPT_EXT: &str = "js";
const TYPESCRIPT_EXT: &str = "ts";

impl ParsableLanguage {
    #[tracing::instrument(name = "get language from filename")]
    pub fn from_filename(filename: &str) -> Option<Self> {
        if let Some(split) = filename.rsplit_once('.') {
            return Some(match split.1 {
                GO_EXT => Self::Go(tree_sitter_go::language()),
                JAVA_EXT => Self::Java(tree_sitter_java::language()),
                RUST_EXT => Self::Rust(tree_sitter_rust::language()),
                C_EXT => Self::C(tree_sitter_c::language()),
                HTML_EXT => Self::Html(tree_sitter_html::language()),
                CSS_EXT => Self::Css(tree_sitter_css::language()),
                LUA_EXT => Self::Lua(tree_sitter_lua::language()),
                JAVASCRIPT_EXT => Self::Javascript(tree_sitter_javascript::language()),
                // MARKDOWN_EXT => Self::Markdown(tree_sitter_markdown::language()),
                // TYPESCRIPT_EXT => Self::Typescript(tree_sitter_typescript::language()),
                _ => Self::None,
            });
        }
        None
    }

    pub fn inner(&self) -> Option<&Language> {
        match self {
            Self::C(l) => Some(l),
            Self::Go(l) => Some(l),
            Self::Rust(l) => Some(l),
            Self::Java(l) => Some(l),
            Self::Javascript(l) => Some(l),
            Self::Html(l) => Some(l),
            Self::Css(l) => Some(l),
            Self::Lua(l) => Some(l),
            Self::None => None,
        }
    }
}

mod tests {
    use tree_sitter::Parser;

    use super::ParsableLanguage;

    #[test]
    fn parses_rust_comments_correctly() {
        let mut parser = Parser::new();
        parser
            .set_language(
                ParsableLanguage::from_filename("main.rs")
                    .unwrap()
                    .inner()
                    .unwrap(),
            )
            .unwrap();

        let input = r#"
            // Comment 
            fn main() { 
                let i = 5;
            }
        "#;

        // let mut all_comments = vec![];
        let mut tree = parser.parse(input, None).unwrap();
        let mut cursor = tree.walk();
        println!("{:?}", cursor.node().kind());
        while cursor.goto_first_child() {
            while cursor.goto_next_sibling() {
                println!("went to next");
                println!("{:?}", cursor.node().kind());
                println!("{:?}", cursor.node().is_extra());
            }
            println!("went to next");
            println!("{:?}", cursor.node().kind());
            println!("{:?}", cursor.node().is_extra());
        }

        assert!(false);
    }
}
