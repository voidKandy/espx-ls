use std::collections::HashMap;

use crate::burns::{tests::mock_burns, Burn, InBufferBurn};

use super::{
    burns::{BurnCache, BurnMap},
    GlobalCache,
};
use lsp_types::{Position, Url};

fn test_burn_cache() -> BurnCache {
    let test_url0 = Url::parse("file:///test/0").unwrap();
    let test_url1 = Url::parse("file:///test/1").unwrap();

    let mut burn_map0 = HashMap::new();
    let mut burn_map1 = HashMap::new();
    let burns = mock_burns();

    for burn in burns {
        match burn.range().start.line {
            1 => burn_map0.insert(
                1,
                InBufferBurn {
                    url: test_url0.clone(),
                    burn,
                },
            ),
            2 => burn_map0.insert(
                2,
                InBufferBurn {
                    url: test_url0.clone(),
                    burn,
                },
            ),
            3 => burn_map1.insert(
                3,
                InBufferBurn {
                    url: test_url1.clone(),
                    burn,
                },
            ),
            4 => burn_map1.insert(
                4,
                InBufferBurn {
                    url: test_url1.clone(),
                    burn,
                },
            ),
            _ => unreachable!(),
        };
    }

    let mut cache = BurnCache::default();

    cache.map.insert(test_url0, burn_map0);
    cache.map.insert(test_url1, burn_map1);
    cache
}

#[test]
fn get_burn_by_position_works() {
    let test_url0 = Url::parse("file:///test/0").unwrap();
    let test_url1 = Url::parse("file:///test/1").unwrap();

    let mut cache = test_burn_cache();

    assert_eq!(
        "@",
        cache
            .get_burn_by_position(
                &test_url0,
                Position {
                    line: 1,
                    character: 2
                }
            )
            .unwrap()
            .burn
            .echo_placeholder()
            .unwrap()
            .as_str()
    );

    assert_eq!(
        "&",
        cache
            .get_burn_by_position(
                &test_url0,
                Position {
                    line: 2,
                    character: 2
                }
            )
            .unwrap()
            .burn
            .echo_placeholder()
            .unwrap()
            .as_str()
    );

    assert_eq!(
        "%",
        cache
            .get_burn_by_position(
                &test_url1,
                Position {
                    line: 3,
                    character: 2
                }
            )
            .unwrap()
            .burn
            .echo_placeholder()
            .unwrap()
            .as_str()
    );

    if let Burn::Action(_) = &cache
        .get_burn_by_position(
            &test_url1,
            Position {
                line: 4,
                character: 2,
            },
        )
        .unwrap()
        .burn
    {
    } else {
        assert!(false)
    }
}
