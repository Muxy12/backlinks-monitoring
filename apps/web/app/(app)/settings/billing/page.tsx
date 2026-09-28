"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api, orgApi, type MeResponse } from "@/lib/api";
import "@/app/auth.css";
import "../../dashboard/dashboard.css";

// Mirrors crates/billing/src/plans.rs (§39). Display-only; the server is the
// source of truth for entitlements.
const PLANS: {
  slug: string;
  name: string;
  price: string;
  backlinks: string;
  projects: string;
  seats: string;
  featured?: boolean;
}[] = [
  { slug: "starter", name: "Starter", price: "$19", backlinks: "1,000", projects: "3", seats: "1" },
  { slug: "pro", name: "Pro", price: "$49", backlinks: "5,000", projects: "20", seats: "3", featured: true },
  { slug: "agency", name: "Agency", price: "$99", backlinks: "20,000", projects: "100", seats: "10" },
  { slug: "agency_plus", name: "Agency+", price: "$199", backlinks: "75,000", projects: "Unlimited", seats: "Unlimited" },
];

export default function BillingPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [plan, setPlan] = useState("starter");
  const [status, setStatus] = useState("none");
  const [configured, setConfigured] = useState(false);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);

  const isOwner = !!me?.organizations.find((o) => o.id === orgId && o.role === "owner");

  const loadStatus = useCallback(async (org: string) => {
    const res = await orgApi.billingStatus(org);
    if (res.ok) {
      setPlan(res.data.plan);
      setStatus(res.data.status);
      setConfigured(res.data.configured);
    }
  }, []);

  useEffect(() => {
    api.me().then((res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      setMe(res.data);
      const org = res.data.organizations[0]?.id ?? null;
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
    const res = await orgApi.startCheckout(orgId, slug);
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

      {!configured && (
        <div className="panel" style={{ marginBottom: 16 }}>
          <p className="muted" style={{ margin: 0 }}>
            Billing isn’t configured on this server yet (Stripe keys missing), so checkout is unavailable.
          </p>
        </div>
      )}
      {msg && <p className="muted" style={{ marginBottom: 12 }}>{msg}</p>}

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
                {p.price}
                <span style={{ fontSize: 13, fontWeight: 400, color: "var(--muted)" }}> /mo</span>
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
