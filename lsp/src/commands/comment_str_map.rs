use core::panic;
use std::{collections::HashMap, sync::LazyLock};

use super::{CommandError, CommandResult};

#[derive(Debug, Clone)]
pub(super) struct CommentStrInfo {
    singleline: String,
    multiline: Option<MultilineCommentInfo>,
}

#[derive(Debug, Clone)]
pub(super) struct MultilineCommentInfo {
    start: String,
    end: String,
}

impl CommentStrInfo {
    pub fn singleline(&self) -> &str {
        &self.singleline
    }

    pub fn multiline_start(&self) -> Option<&str> {
        Some(&self.multiline.as_ref()?.start)
    }

    pub fn multiline_end(&self) -> Option<&str> {
        Some(&self.multiline.as_ref()?.end)
    }
    /// at which char idx does the multiline start differ from a singleline
    pub fn difference_idx(&self) -> Option<usize> {
        let multi = self.multiline_start()?;
        let single = self.singleline();

        for (i, (mchar, schar)) in multi.chars().zip(single.chars()).enumerate() {
            if mchar != schar {
                return Some(i);
            }
        }

        tracing::error!("start to multiline is the same as a singleline");
        None
    }
}

pub(super) fn get_comment_string_info(ext: &str) -> CommandResult<CommentStrInfo> {
    let m = COMMENT_EXTENSION_MAP;
    let map = LazyLock::force(&m);
    let comment_str = map
        .get(ext)
        .ok_or(CommandError::UnhandledLanguageExtension(ext.to_string()))?;
    Ok(comment_str.clone())
}

const COMMENT_EXTENSION_MAP: LazyLock<HashMap<&str, CommentStrInfo>> = LazyLock::new(|| {
    let mut map = HashMap::new();

    map.insert(
        "rs",
        CommentStrInfo {
            singleline: "//".to_string(),
            multiline: Some(MultilineCommentInfo {
                start: "/*".to_string(),
                end: "*/".to_string(),
            }),
        },
    );

    map.insert(
        "c",
        CommentStrInfo {
            singleline: "//".to_string(),
            multiline: Some(MultilineCommentInfo {
                start: "/*".to_string(),
                end: "*/".to_string(),
            }),
        },
    );

    map.insert(
        "cpp",
        CommentStrInfo {
            singleline: "//".to_string(),
            multiline: Some(MultilineCommentInfo {
                start: "/*".to_string(),
                end: "*/".to_string(),
            }),
        },
    );

    map.insert(
        "java",
        CommentStrInfo {
            singleline: "//".to_string(),
            multiline: Some(MultilineCommentInfo {
                start: "/*".to_string(),
                end: "*/".to_string(),
            }),
        },
    );

    map.insert(
        "js",
        CommentStrInfo {
            singleline: "//".to_string(),
            multiline: Some(MultilineCommentInfo {
                start: "/*".to_string(),
                end: "*/".to_string(),
            }),
        },
    );

    map.insert(
        "py",
        CommentStrInfo {
            singleline: "#".to_string(),
            multiline: None,
        },
    );

    map.insert(
        "rb",
        CommentStrInfo {
            singleline: "#".to_string(),
            multiline: None,
        },
    );

    map.insert(
        "php",
        CommentStrInfo {
            singleline: "//".to_string(),
            multiline: Some(MultilineCommentInfo {
                start: "/*".to_string(),
                end: "*/".to_string(),
            }),
        },
    );

    map.insert(
        "cs",
        CommentStrInfo {
            singleline: "//".to_string(),
            multiline: Some(MultilineCommentInfo {
                start: "/*".to_string(),
                end: "*/".to_string(),
            }),
        },
    );

    map.insert(
        "swift",
        CommentStrInfo {
            singleline: "//".to_string(),
            multiline: Some(MultilineCommentInfo {
                start: "/*".to_string(),
                end: "*/".to_string(),
            }),
        },
    );

    map.insert(
        "kt",
        CommentStrInfo {
            singleline: "//".to_string(),
            multiline: Some(MultilineCommentInfo {
                start: "/*".to_string(),
                end: "*/".to_string(),
            }),
        },
    );

    map.insert(
        "pl",
        CommentStrInfo {
            singleline: "#".to_string(),
            multiline: None,
        },
    );

    map.insert(
        "sh",
        CommentStrInfo {
            singleline: "#".to_string(),
            multiline: None,
        },
    );

    map.insert(
        "lua",
        CommentStrInfo {
            singleline: "--".to_string(),
            multiline: Some(MultilineCommentInfo {
                start: "--[[".to_string(),
                end: "]]".to_string(),
            }),
        },
    );

    map.insert(
        "hs",
        CommentStrInfo {
            singleline: "--".to_string(),
            multiline: None,
        },
    );

    map.insert(
        "erl",
        CommentStrInfo {
            singleline: "%".to_string(),
            multiline: None,
        },
    );

    map.insert(
        "ex",
        CommentStrInfo {
            singleline: "#".to_string(),
            multiline: None,
        },
    );

    map.insert(
        "html",
        CommentStrInfo {
            singleline: "<!--".to_string(),
            multiline: None,
        },
    );

    map.insert(
        "xml",
        CommentStrInfo {
            singleline: "<!--".to_string(),
            multiline: None,
        },
    );

    map.insert(
        "sql",
        CommentStrInfo {
            singleline: "--".to_string(),
            multiline: None,
        },
    );

    map.insert(
        "v",
        CommentStrInfo {
            singleline: "//".to_string(),
            multiline: Some(MultilineCommentInfo {
                start: "/*".to_string(),
                end: "*/".to_string(),
            }),
        },
    );

    map.insert(
        "go",
        CommentStrInfo {
            singleline: "//".to_string(),
            multiline: Some(MultilineCommentInfo {
                start: "/*".to_string(),
                end: "*/".to_string(),
            }),
        },
    );

    map.insert(
        "d",
        CommentStrInfo {
            singleline: "//".to_string(),
            multiline: Some(MultilineCommentInfo {
                start: "/*".to_string(),
                end: "*/".to_string(),
            }),
        },
    );

    map.insert(
        "scala",
        CommentStrInfo {
            singleline: "//".to_string(),
            multiline: Some(MultilineCommentInfo {
                start: "/*".to_string(),
                end: "*/".to_string(),
            }),
        },
    );

    map.insert(
        "sh",
        CommentStrInfo {
            singleline: "#".to_string(),
            multiline: None,
        },
    );

    map.insert(
        "r",
        CommentStrInfo {
            singleline: "#".to_string(),
            multiline: None,
        },
    );

    map.insert(
        "cob",
        CommentStrInfo {
            singleline: "*".to_string(),
            multiline: None,
        },
    );

    map.insert(
        "f90",
        CommentStrInfo {
            singleline: "!".to_string(),
            multiline: None,
        },
    );

    map
});

mod tests {
    use super::get_comment_string_info;

    #[test]
    fn diff_idx_works() {
        let info = get_comment_string_info("rs").unwrap();
        assert_eq!(info.difference_idx(), Some(1));
    }
}
