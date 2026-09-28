//! Stripe Checkout + Customer Portal via the REST API (§39). We call Stripe
//! directly with form-encoded POSTs rather than pull the heavy async-stripe SDK.
//! All of it is gated on `STRIPE_SECRET_KEY` and per-plan `STRIPE_PRICE_*` env
//! vars; absent, the caller reports "billing not configured" instead of acting.
//! Stripe's own endpoint is a fixed trusted host, so no SSRF guard is needed
//! (unlike user-supplied webhook URLs).

use crate::plans::Plan;

fn nonempty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}

pub fn secret_key() -> Option<String> {
    nonempty("STRIPE_SECRET_KEY")
}

pub fn is_configured() -> bool {
    secret_key().is_some()
}

/// The Stripe Price id configured for a plan (env `STRIPE_PRICE_<PLAN>`).
pub fn price_for_plan(plan: Plan) -> Option<String> {
    let key = match plan {
        Plan::Starter => "STRIPE_PRICE_STARTER",
        Plan::Pro => "STRIPE_PRICE_PRO",
        Plan::Agency => "STRIPE_PRICE_AGENCY",
        Plan::AgencyPlus => "STRIPE_PRICE_AGENCY_PLUS",
    };
    nonempty(key)
}

/// Reverse map a Stripe Price id back to a plan (for webhook state application).
pub fn plan_from_price(price_id: &str) -> Option<Plan> {
    Plan::ALL
        .into_iter()
        .find(|p| price_for_plan(*p).as_deref() == Some(price_id))
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .unwrap_or_default()
}

async fn post_form(
    secret: &str,
    url: &str,
    form: &[(&str, String)],
) -> Result<serde_json::Value, String> {
    let resp = client()
        .post(url)
        .bearer_auth(secret)
        .form(form)
        .send()
        .await
        .map_err(|e| format!("stripe request failed: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    if status.is_success() {
        Ok(json)
    } else {
        let msg = json["error"]["message"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| text.chars().take(200).collect());
        Err(format!("stripe HTTP {status}: {msg}"))
    }
}

/// Create a subscription Checkout Session for `price`, tagged with the org id so
/// the webhook can attribute the resulting subscription. Returns the hosted URL.
#[allow(clippy::too_many_arguments)]
pub async fn create_checkout_session(
    secret: &str,
    price: &str,
    plan_slug: &str,
    org_id: &str,
    email: &str,
    success_url: &str,
    cancel_url: &str,
) -> Result<String, String> {
    let form: Vec<(&str, String)> = vec![
        ("mode", "subscription".into()),
        ("line_items[0][price]", price.to_string()),
        ("line_items[0][quantity]", "1".into()),
        ("success_url", success_url.to_string()),
        ("cancel_url", cancel_url.to_string()),
        ("client_reference_id", org_id.to_string()),
        ("customer_email", email.to_string()),
        ("allow_promotion_codes", "true".into()),
        ("metadata[organization_id]", org_id.to_string()),
        ("metadata[plan]", plan_slug.to_string()),
        (
            "subscription_data[metadata][organization_id]",
            org_id.to_string(),
        ),
        ("subscription_data[metadata][plan]", plan_slug.to_string()),
    ];
    let json = post_form(secret, "https://api.stripe.com/v1/checkout/sessions", &form).await?;
    json["url"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "stripe: no checkout url in response".to_string())
}

/// Create a Customer Portal session for an existing customer. Returns the URL.
pub async fn create_portal_session(
    secret: &str,
    customer_id: &str,
    return_url: &str,
) -> Result<String, String> {
    let form: Vec<(&str, String)> = vec![
        ("customer", customer_id.to_string()),
        ("return_url", return_url.to_string()),
    ];
    let json = post_form(
        secret,
        "https://api.stripe.com/v1/billing_portal/sessions",
        &form,
    )
    .await?;
    json["url"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "stripe: no portal url in response".to_string())
}
