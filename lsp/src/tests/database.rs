use crate::state::{
    burns::{Burn, MultiLineActivation, MultiLineVariant, SingleLineActivation, SingleLineVariant},
    database::models::{
        DBBurn, DBBurnParams, DBChunk, DBChunkParams, DatabaseStruct, FieldQuery, QueryBuilder,
    },
};
use lsp_types::{Range, Uri};
use std::str::FromStr;

#[tokio::test]
async fn burns_crud_test() {
    super::init_test_tracing();
    let db = super::test_db().await;
    let _: Vec<DBBurn> = db.client.delete(DBBurn::db_id()).await.unwrap();

    let mut query = QueryBuilder::begin();
    let burns = setup_burns();
    for params in burns.clone() {
        query.push(&DBBurn::create(params).unwrap());
    }
    db.client.query(query.end()).await.unwrap();

    let all: Vec<DBBurn> = db.client.select(DBBurn::db_id()).await.unwrap();
    assert_eq!(all.len(), burns.len());

    let mut query = QueryBuilder::begin();
    let first = burns[0].clone();
    let len_not_that_uri = burns
        .iter()
        .fold(0, |acc, ch| if ch.uri != first.uri { acc + 1 } else { acc });

    let uri = first.uri.as_ref().unwrap().as_str();
    query.push(&DBBurn::delete(FieldQuery::new("uri", uri).unwrap()).unwrap());
    db.client.query(query.end()).await.unwrap();

    let all: Vec<DBBurn> = db.client.select(DBBurn::db_id()).await.unwrap();
    assert_eq!(all.len(), len_not_that_uri);

    let uri = all[0].uri.as_str();
    let fq = FieldQuery::new("uri", uri).unwrap();

    let all_to_update: Vec<DBBurn> = db
        .client
        .query(DBBurn::select(Some(fq), None).unwrap())
        .await
        .unwrap()
        .take(0)
        .unwrap();

    let b = Burn::from(SingleLineActivation::new(
        SingleLineVariant::QuickPrompt,
        "#$",
        Range {
            start: lsp_types::Position {
                line: 0,
                character: 0,
            },

            end: lsp_types::Position {
                line: 0,
                character: 2,
            },
        },
    ));

    let mut q = QueryBuilder::begin();

    for mut dbb in all_to_update {
        dbb.burn = b.clone();
        q.push(&DBBurn::update(dbb.thing().clone(), dbb).unwrap());
    }
    let updated: Vec<DBBurn> = db.client.query(&q.end()).await.unwrap().take(0).unwrap();

    for ch in updated {
        assert_eq!(ch.burn.hover_contents, b.hover_contents);
        assert_eq!(ch.burn.activation, b.activation);
    }
}

#[tokio::test]
async fn chunks_crud_test() {
    super::init_test_tracing();
    let db = super::test_db().await;
    let _: Vec<DBChunk> = db.client.delete(DBChunk::db_id()).await.unwrap();

    let mut query = QueryBuilder::begin();
    let chunks = setup_chunks();
    for params in chunks.clone() {
        query.push(&DBChunk::create(params).unwrap());
    }
    db.client.query(query.end()).await.unwrap();

    let all: Vec<DBChunk> = db.client.select(DBChunk::db_id()).await.unwrap();
    assert_eq!(all.len(), chunks.len());

    let mut query = QueryBuilder::begin();
    let first = chunks[0].clone();
    let len_not_that_uri = chunks
        .iter()
        .fold(0, |acc, ch| if ch.uri != first.uri { acc + 1 } else { acc });

    let uri = first.uri.as_ref().unwrap().as_str();
    query.push(&DBChunk::delete(FieldQuery::new("uri", uri).unwrap()).unwrap());
    db.client.query(query.end()).await.unwrap();

    let all: Vec<DBChunk> = db.client.select(DBChunk::db_id()).await.unwrap();
    assert_eq!(all.len(), len_not_that_uri);

    let uri = all[0].uri.as_str();
    let fq = FieldQuery::new("uri", uri).unwrap();

    let update_params = DBChunkParams {
        content: Some("new content!!!".to_string()),
        ..Default::default()
    };

    db.client
        .query(DBChunk::update(fq.clone(), update_params).unwrap())
        .await
        .unwrap();

    let updated: Vec<DBChunk> = db
        .client
        .query(DBChunk::select(Some(fq), None).unwrap())
        .await
        .unwrap()
        .take(0)
        .unwrap();

    for ch in updated {
        assert_eq!(ch.content, String::from("new content!!!"));
        assert_eq!(ch.uri.as_str(), uri);
    }

    let _ = DBChunk::get_relavent(&db, [1., 2., 3., 4., 5.].to_vec(), 0.5)
        .await
        .unwrap();
}

fn setup_burns() -> Vec<DBBurnParams> {
    vec![
        DBBurnParams {
            burn: Some(Burn::from(SingleLineActivation::new(
                SingleLineVariant::QuickPrompt,
                "#$",
                Range {
                    start: lsp_types::Position {
                        line: 0,
                        character: 0,
                    },

                    end: lsp_types::Position {
                        line: 0,
                        character: 2,
                    },
                },
            ))),
            uri: Some(Uri::from_str("file:///tmp/foo").unwrap()),
        },
        DBBurnParams {
            burn: Some(Burn::from(SingleLineActivation::new(
                SingleLineVariant::RagPrompt,
                "#$#",
                Range {
                    start: lsp_types::Position {
                        line: 1,
                        character: 0,
                    },

                    end: lsp_types::Position {
                        line: 1,
                        character: 3,
                    },
                },
            ))),
            uri: Some(Uri::from_str("file:///tmp/bar").unwrap()),
        },
        DBBurnParams {
            burn: Some(Burn::from(MultiLineActivation {
                variant: MultiLineVariant::LockChunkIntoContext,
                start_range: Range {
                    start: lsp_types::Position {
                        line: 1,
                        character: 0,
                    },

                    end: lsp_types::Position {
                        line: 1,
                        character: 7,
                    },
                }
                .into(),
                end_range: Range {
                    start: lsp_types::Position {
                        line: 3,
                        character: 0,
                    },

                    end: lsp_types::Position {
                        line: 1,
                        character: 7,
                    },
                }
                .into(),
            })),
            uri: Some(Uri::from_str("file:///tmp/baz").unwrap()),
        },
    ]
}

fn setup_chunks() -> Vec<DBChunkParams> {
    vec![
        DBChunkParams {
            uri: Some(Uri::from_str("file:///tmp/foo").unwrap()),
            content: Some("Chunk 1 of foo\n".to_string()),
            content_embedding: Some(vec![1., 2., 3., 4., 5.]),
            range: Some((0, 1)),
        },
        DBChunkParams {
            uri: Some(Uri::from_str("file:///tmp/foo").unwrap()),
            content: Some("Chunk 2 of foo\n".to_string()),
            content_embedding: Some(vec![1., 2., 3., 4., 5.]),
            range: Some((1, 2)),
        },
        DBChunkParams {
            uri: Some(Uri::from_str("file:///tmp/foo").unwrap()),
            content: Some("Chunk 3 of foo\n".to_string()),
            content_embedding: Some(vec![1., 2., 3., 4., 5.]),
            range: Some((2, 3)),
        },
        DBChunkParams {
            uri: Some(Uri::from_str("file:///tmp/bar").unwrap()),
            content: Some("Chunk 1 of bar\n".to_string()),
            content_embedding: Some(vec![1., 2., 3., 4., 5.]),
            range: Some((0, 1)),
        },
        DBChunkParams {
            uri: Some(Uri::from_str("file:///tmp/bar").unwrap()),
            content: Some("Chunk 2 of bar\n".to_string()),
            content_embedding: Some(vec![1., 2., 3., 4., 5.]),
            range: Some((1, 2)),
        },
        DBChunkParams {
            uri: Some(Uri::from_str("file:///tmp/bar").unwrap()),
            content: Some("Chunk 3 of bar\n".to_string()),
            content_embedding: Some(vec![1., 2., 3., 4., 5.]),
            range: Some((2, 3)),
        },
        DBChunkParams {
            uri: Some(Uri::from_str("file:///tmp/baz").unwrap()),
            content: Some("Chunk 1 of baz\n".to_string()),
            content_embedding: Some(vec![1., 2., 3., 4., 5.]),
            range: Some((0, 1)),
        },
        DBChunkParams {
            uri: Some(Uri::from_str("file:///tmp/baz").unwrap()),
            content: Some("Chunk 2 of baz\n".to_string()),
            content_embedding: Some(vec![1., 2., 3., 4., 5.]),
            range: Some((1, 2)),
        },
        DBChunkParams {
            uri: Some(Uri::from_str("file:///tmp/baz").unwrap()),
            content: Some("Chunk 3 of baz\n".to_string()),
            content_embedding: Some(vec![1., 2., 3., 4., 5.]),
            range: Some((2, 3)),
        },
    ]
}
