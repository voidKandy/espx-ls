use super::{multiline::MultiLineBurn, singleline::SingleLineBurn};
use lsp_types::OneOf;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum BurnActivation {
    Single(SingleLineBurn),
    Multi(MultiLineBurn),
}

impl From<SingleLineBurn> for BurnActivation {
    fn from(value: SingleLineBurn) -> Self {
        Self::Single(value)
    }
}

impl From<MultiLineBurn> for BurnActivation {
    fn from(value: MultiLineBurn) -> Self {
        Self::Multi(value)
    }
}

type Single = SingleLineBurn;
type Multi = MultiLineBurn;

impl BurnActivation {
    pub fn into_inner(self) -> OneOf<Single, Multi> {
        match self {
            Self::Single(s) => OneOf::Left(s),
            Self::Multi(m) => OneOf::Right(m),
        }
    }
}
