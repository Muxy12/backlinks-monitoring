//! HTTP route handlers. Thin: validate input, call a crate, map to a response
//! (docs/02 §apps). Business rules live in the crates, not here.

pub mod admin_routes;
pub mod alerts_routes;
pub mod auth_routes;
pub mod billing_routes;
pub mod campaigns_routes;
pub mod competitor_routes;
pub mod discovery_routes;
pub mod feedback_routes;
pub mod graph_routes;
pub mod gsc_routes;
pub mod impact_routes;
pub mod internal_routes;
pub mod me_routes;
pub mod ops_routes;
pub mod org_routes;
pub mod overview_routes;
pub mod prefs_routes;
pub mod resource_routes;
pub mod tags_routes;
pub mod team_routes;
pub mod tools_routes;
pub mod waitlist_routes;
