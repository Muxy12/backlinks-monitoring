//! Backlio shared foundation: environment, config, IDs, errors, clock.
//! The leaf crate — everything may depend on it, it depends on nothing internal
//! (ADR-0001).

pub mod env;
pub mod ids;

pub use env::{AppEnv, Guards};
pub use ids::{
    BacklinkId, CampaignId, ClientId, JobId, OrganizationId, ProjectId, PublisherId, UserId,
};
