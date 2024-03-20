use std::{collections::HashMap, time::SystemTime};

// #[derive(Debug, Clone)]
// pub struct ActionStore(Vec<(EmbeddingVector, Url, Action)>);
//
// #[derive(Debug, Clone)]
// pub struct Action {
//     summary: String,
//     timestamp: SystemTime,
// }
//
// impl Default for ActionStore {
//     fn default() -> Self {
//         Self(vec![])
//     }
// }
//
// impl ActionStore {
//     pub fn get_by_proximity(
//         &self,
//         input_vector: EmbeddingVector,
//         proximity: f32,
//     ) -> HashMap<&Url, &Action> {
//         let mut map = HashMap::new();
//         self.0.iter().for_each(|(e, url, action)| {
//             if input_vector.score_l2(e) <= proximity {
//                 map.insert(url, action);
//             }
//         });
//         map
//     }
//
//     pub async fn insert_action(&mut self, action: Action, url: Url) -> Result<(), anyhow::Error> {
//         // NEED CLIENT
//
//         let response = get_embedding(&action.summary).await?;
//
//         self.0.push((embedding, url, action));
//         Ok(())
//     }
// }
