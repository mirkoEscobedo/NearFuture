#![no_std]
extern crate alloc;
mod model;
mod validation;
mod war;
pub use model::*;
pub use war::evaluate_war;
mod project;
pub use project::{evaluate_world_war, selected_peace_from_world, war_from_world};
mod peace;
mod peace_model;
pub use peace::evaluate_selected_peace;
pub use peace_model::*;
mod identity;
mod session;
pub use identity::{
    ShadowInput, ShadowMetadata, action_input_digest, encode_action_input, encode_input,
    input_digest,
};
pub use session::*;
mod diagnostics;
pub use diagnostics::*;
mod world_session;
pub use world_session::*;
mod world_commitment;
pub use world_commitment::{WorldCommitment, commit_world};

mod action;
pub use action::{MakePeaceEligibility, make_peace_action_eligible};

mod traversal;
pub use traversal::{
    PeaceTraversalResult, ReportedPeaceReturn, TraversalEnemy, TraversalStep, TraversalVisit,
    evaluate_passed_outer_gates, evaluate_selection_passed_outer_gates,
};

mod action_priority;
pub use action_priority::make_peace_weariness_modifier;

mod first_proposal;
pub use first_proposal::{FirstProposalEvaluation, FirstProposalInput, FirstProposalOutput};
pub use identity::{encode_first_proposal_input, first_proposal_input_digest};
