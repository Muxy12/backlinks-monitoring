//! `APP_ENV` and the guards that gate dangerous behaviour by environment
//! (docs/10 §1, §44). These live in ONE place and are asserted by tests, so a
//! new dangerous capability cannot be added without a gate — a gate scattered
//! across a dozen `if`s is a gate that will be missed once.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// The deployment environment. Explicit — never inferred from `NODE_ENV`,
/// `VERCEL_ENV` or the presence of a DSN (§44).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppEnv {
    Development,
    Preview,
    Production,
}

impl FromStr for AppEnv {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "development" | "dev" => Ok(AppEnv::Development),
            "preview" => Ok(AppEnv::Preview),
            "production" | "prod" => Ok(AppEnv::Production),
            other => Err(format!(
                "invalid APP_ENV {other:?}: expected development | preview | production"
            )),
        }
    }
}

impl fmt::Display for AppEnv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            AppEnv::Development => "development",
            AppEnv::Preview => "preview",
            AppEnv::Production => "production",
        })
    }
}

/// The single source of truth for which dangerous capabilities are permitted in
/// which environment (docs/10 §1). Every dangerous action calls through here.
///
/// The table this encodes:
///
/// | capability            | development | preview | production |
/// |-----------------------|:-----------:|:-------:|:----------:|
/// | process jobs          |   opt-in    |   no    |    yes     |
/// | outbound crawling     |   opt-in    |   no    |    yes     |
/// | send real email       |    no       |   no    |    yes     |
/// | accept scheduler tick |    yes      |   no    |    yes     |
/// | live Stripe keys      |    no       |   no    |    yes     |
/// | seed/demo endpoints   |    yes      |   yes   |    no      |
#[derive(Debug, Clone, Copy)]
pub struct Guards {
    env: AppEnv,
    /// In development only, jobs/crawling are off unless explicitly opted in
    /// (so a laptop does not crawl the web by accident).
    dev_opt_in: bool,
}

impl Guards {
    pub fn new(env: AppEnv, dev_opt_in: bool) -> Self {
        Guards { env, dev_opt_in }
    }

    pub fn env(&self) -> AppEnv {
        self.env
    }

    /// Preview NEVER processes monitoring jobs (§44).
    pub fn may_process_jobs(&self) -> bool {
        match self.env {
            AppEnv::Production => true,
            AppEnv::Preview => false,
            AppEnv::Development => self.dev_opt_in,
        }
    }

    /// Preview NEVER crawls (§44). Tied to job processing — you cannot crawl
    /// without running a job.
    pub fn may_crawl(&self) -> bool {
        self.may_process_jobs()
    }

    /// Real email goes out in production only; dev captures to Mailpit, preview
    /// captures too (§44, docs/10 §1).
    pub fn may_send_real_email(&self) -> bool {
        self.env == AppEnv::Production
    }

    /// The scheduler tick is rejected in preview so a PR deploy cannot enqueue
    /// production-shaped work (§44).
    pub fn may_accept_scheduler_tick(&self) -> bool {
        self.env != AppEnv::Preview
    }

    /// Live Stripe keys are production-only; everywhere else uses test keys
    /// (§44).
    pub fn may_use_live_stripe(&self) -> bool {
        self.env == AppEnv::Production
    }

    /// Seed / demo-data endpoints must not exist in production (§44).
    pub fn may_expose_seed_endpoints(&self) -> bool {
        self.env != AppEnv::Production
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_and_aliases_case_insensitively() {
        assert_eq!("production".parse(), Ok(AppEnv::Production));
        assert_eq!("PROD".parse(), Ok(AppEnv::Production));
        assert_eq!("  Development ".parse(), Ok(AppEnv::Development));
        assert_eq!("preview".parse(), Ok(AppEnv::Preview));
        assert!("staging".parse::<AppEnv>().is_err());
    }

    #[test]
    fn preview_can_do_nothing_dangerous() {
        let g = Guards::new(AppEnv::Preview, true /* opt-in must be ignored */);
        assert!(
            !g.may_process_jobs(),
            "preview must never process jobs (§44)"
        );
        assert!(!g.may_crawl(), "preview must never crawl (§44)");
        assert!(!g.may_send_real_email());
        assert!(
            !g.may_accept_scheduler_tick(),
            "preview must reject the scheduler tick"
        );
        assert!(!g.may_use_live_stripe());
        // ...but preview CAN expose seed endpoints (it is not production).
        assert!(g.may_expose_seed_endpoints());
    }

    #[test]
    fn production_does_the_real_work_but_no_seed_endpoints() {
        let g = Guards::new(AppEnv::Production, false);
        assert!(g.may_process_jobs());
        assert!(g.may_crawl());
        assert!(g.may_send_real_email());
        assert!(g.may_accept_scheduler_tick());
        assert!(g.may_use_live_stripe());
        assert!(
            !g.may_expose_seed_endpoints(),
            "no seed/demo endpoints in production (§44)"
        );
    }

    #[test]
    fn development_gates_jobs_behind_explicit_opt_in() {
        let off = Guards::new(AppEnv::Development, false);
        assert!(!off.may_process_jobs(), "dev must not crawl by default");
        assert!(!off.may_crawl());

        let on = Guards::new(AppEnv::Development, true);
        assert!(on.may_process_jobs());
        assert!(on.may_crawl());

        // Never real email or live Stripe in dev, opt-in or not.
        assert!(!on.may_send_real_email());
        assert!(!on.may_use_live_stripe());
    }
}
