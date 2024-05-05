use super::super::DatabaseIdentifier;
use crate::store::burns::BurnMap;
use lsp_types::Url;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DBDocumentInfo {
    pub url: Url,
    pub burns: BurnMap,
}

impl DatabaseIdentifier for DBDocumentInfo {
    fn db_id() -> &'static str {
        "documents"
    }
}

impl ToString for DBDocumentInfo {
    fn to_string(&self) -> String {
        let burns_lines: Vec<&u32> = self.burns.keys().collect();
        let ibbs: Vec<String> = self
            .burns
            .values()
            .map(|ibb| {
                format!(
                    r#"
        RANGE: {:?}
         {}
        "#,
                    ibb.burn.range(),
                    ibb.burn
                        .echo_placeholder()
                        .unwrap_or("IS AN ACTION AVAILABLE TO USER".to_string())
                )
            })
            .collect();

        let burns_content = {
            burns_lines
                .iter()
                .enumerate()
                .fold(String::new(), |mut acc, (i, l)| {
                    acc.push_str(&format!(
                        r#"
                [ BURN ON LINE {} ]
                {} 
                "#,
                        l, ibbs[i]
                    ));
                    acc
                })
        };
        format!(
            r#"
        [ BEGINNING OF DOCUMENT: {} ]

        {}

        [ END OF DOCUMENT: {} ]

        "#,
            self.url, burns_content, self.url,
        )
    }
}
