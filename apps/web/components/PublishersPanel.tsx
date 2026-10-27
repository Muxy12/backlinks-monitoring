"use client";

import { useCallback, useEffect, useState } from "react";
import { orgApi, type PublisherView, type PublisherRollup } from "@/lib/api";

// Publisher directory — the sites / vendors you source links from. Each publisher
// expands to a rollup (total / live / lost / removal rate / lifespan / spend).
// Self-contained: fetches its own data from `orgId`. Lifted out of the retired
// Operations page so publishers live alongside the Link Builder. Styling reuses
// the shared `operations.css` primitives (imported by the host page).
export function PublishersPanel({ orgId }: { orgId: string }) {
  const [publishers, setPublishers] = useState<PublisherView[]>([]);
  const [name, setName] = useState("");
  const [domain, setDomain] = useState("");
  const [contactName, setContactName] = useState("");
  const [contactEmail, setContactEmail] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [rollups, setRollups] = useState<Record<string, PublisherRollup>>({});

  const load = useCallback(async (org: string) => {
    const res = await orgApi.publishers(org);
    if (res.ok) setPublishers(res.data);
  }, []);

  useEffect(() => {
    void load(orgId);
  }, [orgId, load]);

  async function toggle(id: string) {
    if (rollups[id]) {
      setRollups((r) => {
        const c = { ...r };
        delete c[id];
        return c;
      });
      return;
    }
    const res = await orgApi.publisherRollup(orgId, id);
    if (res.ok) setRollups((r) => ({ ...r, [id]: res.data }));
  }

  async function create(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    const res = await orgApi.createPublisher(orgId, {
      name: name.trim(),
      domain: domain.trim(),
      contact_name: contactName.trim() || undefined,
      contact_email: contactEmail.trim() || undefined,
    });
    setBusy(false);
    if (!res.ok) {
      setError(res.detail);
      return;
    }
    setName("");
    setDomain("");
    setContactName("");
    setContactEmail("");
    void load(orgId);
  }

  return (
    <div className="ops-grid">
      <section className="panel">
        <div className="panel-head"><h2>Publishers</h2></div>
        {publishers.length === 0 ? (
          <p className="muted">No publishers yet.</p>
        ) : (
          <ul className="ops-list">
            {publishers.map((p) => (
              <li key={p.id}>
                <button className="ops-row" onClick={() => void toggle(p.id)}>
                  <span className="ops-name">{p.name}</span>
                  <span className="muted mono">{p.domain}</span>
                </button>
                {rollups[p.id] && <PublisherRollupCard r={rollups[p.id]} />}
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="panel">
        <div className="panel-head"><h2>New publisher</h2></div>
        <form onSubmit={create} className="ops-form">
          {error && <div className="auth-error" role="alert">{error}</div>}
          <label className="auth-field">
            <span>Name</span>
            <input value={name} onChange={(e) => setName(e.target.value)} required />
          </label>
          <label className="auth-field">
            <span>Domain</span>
            <input value={domain} onChange={(e) => setDomain(e.target.value)} placeholder="publisher.com" required />
          </label>
          <label className="auth-field">
            <span>Contact name (optional)</span>
            <input value={contactName} onChange={(e) => setContactName(e.target.value)} />
          </label>
          <label className="auth-field">
            <span>Contact email (optional)</span>
            <input type="email" value={contactEmail} onChange={(e) => setContactEmail(e.target.value)} />
          </label>
          <button className="auth-btn" type="submit" disabled={busy}>
            {busy ? "Creating…" : "Create publisher"}
          </button>
        </form>
      </section>
    </div>
  );
}

function PublisherRollupCard({ r }: { r: PublisherRollup }) {
  return (
    <div className="rollup">
      <Stat label="Total" v={r.total} />
      <Stat label="Live" v={r.live} />
      <Stat label="Lost" v={r.lost} />
      <Stat label="Removal" v={pct(r.removal_rate)} />
      <Stat label="Avg lifespan" v={lifespan(r.avg_lifespan_days)} wide />
      <Stat label="Spend" v={money(r.spend)} wide />
    </div>
  );
}

function Stat({ label, v, wide }: { label: string; v: number | string; wide?: boolean }) {
  return (
    <div className={wide ? "stat wide" : "stat"}>
      <div className="stat-label">{label}</div>
      <div className="stat-value">{v}</div>
    </div>
  );
}

function money(spend: { currency: string; amount: string }[]) {
  return spend.length === 0 ? "-" : spend.map((s) => `${s.amount} ${s.currency}`).join(" · ");
}
function pct(x: number | null) {
  return x === null ? "-" : `${Math.round(x * 100)}%`;
}
function lifespan(x: number | null) {
  return x === null ? "-" : `${x.toFixed(1)} days`;
}
