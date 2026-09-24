//! Idempotency-key construction for the enqueue side (docs/07 §4). The unique
//! partial index `jobs_idempotency_uq` rejects a duplicate while an equivalent
//! job is still pending/processing — so pressing "Check now" five times
//! enqueues one job (§36).

use uuid::Uuid;

/// Key for a user-triggered "Check now" on a backlink. Stable for a given
/// backlink, so repeated clicks collapse to one pending job.
pub fn check_now_key(backlink_id: Uuid) -> String {
    format!("check_now:{backlink_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_now_key_is_stable_per_backlink() {
        let id = Uuid::now_v7();
        assert_eq!(check_now_key(id), check_now_key(id));
    }

    #[test]
    fn check_now_key_differs_per_backlink() {
        assert_ne!(check_now_key(Uuid::now_v7()), check_now_key(Uuid::now_v7()));
    }

    #[test]
    fn key_has_the_expected_shape() {
        let id = Uuid::now_v7();
        assert_eq!(check_now_key(id), format!("check_now:{id}"));
    }
}
