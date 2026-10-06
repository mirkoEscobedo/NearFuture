#![no_std]
extern crate alloc;

mod admission;
mod model;
pub use admission::*;
pub use model::*;
mod float;
use alloc::string::String;
pub use float::{DoubleBits, FloatBits};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Definition {
    pub id: String,
    pub class_path: String,
    pub module: Module,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Module {
    Diplomatic,
    Economic,
    Military,
    Executive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Surface {
    WarWearinessInput,
    MakePeaceUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoundaryError {
    NonFinite,
    MissingFact(&'static str),
    InvalidFact(&'static str),
    Limit,
    Duplicate(&'static str),
    UnknownDefinition { id: String, class_path: String },
}

pub fn classify_definition(definition: &Definition) -> Result<Surface, BoundaryError> {
    let mut budget = validation::Budget::default();
    budget.text(&definition.id)?;
    budget.text(&definition.class_path)?;
    match (
        definition.id.as_str(),
        definition.class_path.as_str(),
        definition.module,
    ) {
        (
            "warWeariness",
            "exerelin.campaign.ai.concern.WarWearinessConcern",
            Module::Diplomatic,
        ) => Ok(Surface::WarWearinessInput),
        ("makePeace", "exerelin.campaign.ai.action.MakePeaceAction", Module::Diplomatic) => {
            Ok(Surface::MakePeaceUnavailable)
        }
        _ => Err(BoundaryError::UnknownDefinition {
            id: definition.id.clone(),
            class_path: definition.class_path.clone(),
        }),
    }
}
mod validation;
