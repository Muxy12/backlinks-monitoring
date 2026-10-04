"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api, orgApi, type MeResponse, type UsageView, type UsageItem } from "@/lib/api";
import { selectedOrgId } from "@/lib/workspace";
import "@/app/auth.css";
import "../../dashboard/dashboard.css";
import "./billing.css";

// Mirrors crates/billing/src/plans.rs (§39). Display-only; the server is the
// source of truth for entitlements and prices. `priceAnnual` is the per-month
// equivalent of the annual plan (yearly total ÷ 12); annual = 10× monthly.
const PLANS: {
  slug: string;
  name: string;
  price: string;
  priceAnnual: string;
  annualTotal: string;
  backlinks: string;
  projects: string;
  seats: string;
  featured?: boolean;
}[] = [
  { slug: "starter", name: "Starter", price: "$29", priceAnnual: "$24", annualTotal: "$290", backlinks: "500", projects: "3", seats: "1" },
  { slug: "pro", name: "Pro", price: "$59", priceAnnual: "$49", annualTotal: "$590", backlinks: "2,500", projects: "10", seats: "3", featured: true },
  { slug: "business", name: "Business", price: "$99", priceAnnual: "$82", annualTotal: "$990", backlinks: "5,000", projects: "25", seats: "5" },
  { slug: "agency", name: "Agency", price: "$199", priceAnnual: "$166", annualTotal: "$1,990", backlinks: "15,000", projects: "75", seats: "15" },
];

export default function BillingPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [plan, setPlan] = useState("starter");
  const [status, setStatus] = useState("none");
  const [configured, setConfigured] = useState(false);
  const [trialDaysLeft, setTrialDaysLeft] = useState<number | null>(null);
  const [accessActive, setAccessActive] = useState(true);
  const [usage, setUsage] = useState<UsageView | null>(null);
  const [annual, setAnnual] = useState(false);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);

  const isOwner = !!me?.organizations.find((o) => o.id === orgId && o.role === "owner");

  const loadStatus = useCallback(async (org: string) => {
    const [s, u] = await Promise.all([orgApi.billingStatus(org), orgApi.billingUsage(org)]);
    if (s.ok) {
      setPlan(s.data.plan);
      setStatus(s.data.status);
      setConfigured(s.data.configured);
      setTrialDaysLeft(s.data.trial_days_left ?? null);
      setAccessActive(s.data.access_active);
    }
    if (u.ok) setUsage(u.data);
  }, []);

  useEffect(() => {
    api.me().then((res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      setMe(res.data);
      const org = selectedOrgId(res.data.organizations);
      setOrgId(org);
      setLoading(false);
    });
  }, [router]);

  useEffect(() => {
    if (orgId) void loadStatus(orgId);
  }, [orgId, loadStatus]);

  useEffect(() => {
    const c = new URLSearchParams(window.location.search).get("checkout");
    if (c === "success") setMsg("✓ Subscription started. It may take a few seconds to reflect here.");
    else if (c === "cancel") setMsg("Checkout cancelled.");
  }, []);

  async function subscribe(slug: string) {
    if (!orgId) return;
    setBusy(slug);
    setMsg(null);
    const res = await orgApi.startCheckout(orgId, slug, annual ? "annual" : "monthly");
    setBusy(null);
    if (!res.ok) {
      setMsg(res.detail);
      return;
    }
    window.location.href = res.data.url;
  }

  async function manage() {
    if (!orgId) return;
    setBusy("portal");
    setMsg(null);
    const res = await orgApi.openBillingPortal(orgId);
    setBusy(null);
    if (!res.ok) {
      setMsg(res.detail);
      return;
    }
    window.location.href = res.data.url;
  }

  if (loading) return <div className="auth-wrap"><p style={{ color: "var(--muted)" }}>Loading…</p></div>;
  if (!me) return null;

  return (
    <main className="dash">
      <header className="page-head">
        <h1>Billing</h1>
        <p className="muted">
          Current plan: <b>{plan}</b>
          {status !== "none" && status !== "active" ? ` (${status})` : ""}.
          {isOwner ? "" : " Only an owner can change billing."}
        </p>
      </header>

      {(status === "trialing" || !accessActive) && (
        <div className={`panel trial-note${!accessActive ? " ended" : ""}`} style={{ marginBottom: 16 }}>
          {!accessActive ? (
            <p style={{ margin: 0 }}>
              {status === "none" ? (
                <><b>Choose a plan to start monitoring.</b> Your workspace is ready, pick a plan below to begin.</>
              ) : (
                <><b>Your free trial has ended.</b> Choose a plan below to resume monitoring, your backlinks and history are preserved.</>
              )}
            </p>
          ) : (
            <p style={{ margin: 0 }}>
              <b>
                {trialDaysLeft != null && trialDaysLeft <= 1
                  ? "Last day of your free trial."
                  : `${trialDaysLeft ?? ""} days left in your free trial.`}
              </b>{" "}
              No credit card has been charged. Choose a plan any time to continue after the trial.
            </p>
          )}
        </div>
      )}

      {usage && (
        <div className="panel usage-panel" style={{ marginBottom: 16 }}>
          <div className="usage-head">Usage on your current plan</div>
          <div className="usage-grid">
            <UsageMeter label="Backlinks monitored" item={usage.backlinks} />
            <UsageMeter label="Projects" item={usage.projects} />
            <UsageMeter label="Team seats" item={usage.seats} />
          </div>
          <p className="muted" style={{ fontSize: 12, margin: "4px 0 0" }}>
            Reaching a limit never stops monitoring or deletes data, you just can’t add new ones until you upgrade.
          </p>
        </div>
      )}

      {!configured && (
        <div className="panel" style={{ marginBottom: 16 }}>
          <p className="muted" style={{ margin: 0 }}>
            Billing isn’t configured on this server yet (Stripe keys missing), so checkout is unavailable.
          </p>
        </div>
      )}
      {msg && <p className="muted" style={{ marginBottom: 12 }}>{msg}</p>}

      <div className="bill-interval" role="group" aria-label="Billing period">
        <button
          type="button"
          className={`bi-seg${!annual ? " on" : ""}`}
          aria-pressed={!annual}
          onClick={() => setAnnual(false)}
        >
          Monthly
        </button>
        <button
          type="button"
          className={`bi-seg${annual ? " on" : ""}`}
          aria-pressed={annual}
          onClick={() => setAnnual(true)}
        >
          Annual <span className="bi-save">2 months free</span>
        </button>
      </div>

      <div className="tiles" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))" }}>
        {PLANS.map((p) => {
          const current = p.slug === plan;
          return (
            <div
              className="tile"
              key={p.slug}
              style={
                current
                  ? { borderColor: "var(--accent)", boxShadow: "0 0 0 1px var(--accent) inset" }
                  : p.featured
                    ? { borderColor: "color-mix(in srgb, var(--accent) 40%, var(--border))" }
                    : undefined
              }
            >
              <div className="tile-label" style={{ display: "flex", justifyContent: "space-between" }}>
                <span>{p.name}</span>
                {current && <span style={{ color: "var(--accent-2)" }}>current</span>}
                {!current && p.featured && <span style={{ color: "var(--accent-2)" }}>popular</span>}
              </div>
              <div className="tile-value">
                {annual ? p.priceAnnual : p.price}
                <span style={{ fontSize: 13, fontWeight: 400, color: "var(--muted)" }}> /mo</span>
                {annual && (
                  <div style={{ fontSize: 12, fontWeight: 400, color: "var(--muted)", marginTop: 2 }}>
                    {p.annualTotal} billed yearly
                  </div>
                )}
              </div>
              <ul className="muted" style={{ fontSize: 13, listStyle: "none", padding: 0, margin: "10px 0" }}>
                <li>✓ {p.backlinks} backlinks</li>
                <li>✓ {p.projects} projects</li>
                <li>✓ {p.seats} seat{p.seats === "1" ? "" : "s"}</li>
              </ul>
              <button
                className={`btn sm${p.featured && !current ? " primary" : ""}`}
                style={{ width: "100%" }}
                disabled={!configured || !isOwner || current || busy !== null}
                onClick={() => void subscribe(p.slug)}
              >
                {busy === p.slug ? "…" : current ? "Current plan" : "Subscribe"}
              </button>
            </div>
          );
        })}
      </div>

      <div style={{ marginTop: 20 }}>
        <button className="btn" disabled={!configured || !isOwner || busy !== null} onClick={() => void manage()}>
          {busy === "portal" ? "…" : "Manage billing (invoices, payment method, cancel)"}
        </button>
        <p className="muted" style={{ fontSize: 12, marginTop: 8 }}>
          Opens Stripe’s secure Customer Portal. Available once you have a subscription.
        </p>
      </div>
    </main>
  );
}

function UsageMeter({ label, item }: { label: string; item: UsageItem }) {
  const limit = item.limit;
  if (limit == null) {
    return (
      <div className="usage-item">
        <div className="usage-row">
          <span className="usage-label">{label}</span>
          <span className="usage-count">{item.used.toLocaleString()} · unlimited</span>
        </div>
      </div>
    );
  }
  const pct = limit === 0 ? 0 : Math.min(100, Math.round((item.used / limit) * 100));
  const over = item.used >= limit;
  const high = !over && pct >= 80;
  return (
    <div className="usage-item">
      <div className="usage-row">
        <span className="usage-label">{label}</span>
        <span className={`usage-count${over ? " over" : ""}`}>
          {item.used.toLocaleString()} / {limit.toLocaleString()}
        </span>
      </div>
      <div className="usage-bar" role="progressbar" aria-valuenow={item.used} aria-valuemax={limit}>
        <span className={`usage-fill${over ? " over" : high ? " high" : ""}`} style={{ width: `${pct}%` }} />
      </div>
    </div>
  );
}
