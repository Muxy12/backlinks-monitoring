"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import {
  api,
  orgApi,
  type CampaignRollup,
  type CampaignView,
  type MeResponse,
  type ProjectView,
  type PublisherRollup,
  type PublisherView,
} from "@/lib/api";
import { selectedOrg, selectedOrgId } from "@/lib/workspace";
import "@/app/auth.css";
import "../dashboard/dashboard.css";
import "./operations.css";

type Tab = "campaigns" | "publishers";

export default function OperationsPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>("campaigns");
  const [projects, setProjects] = useState<ProjectView[]>([]);
  const [campaigns, setCampaigns] = useState<CampaignView[]>([]);
  const [publishers, setPublishers] = useState<PublisherView[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.me().then((res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      setMe(res.data);
      setOrgId(selectedOrgId(res.data.organizations));
      setLoading(false);
    });
  }, [router]);

  const reload = useCallback(async (org: string) => {
    const [projs, camps, pubs] = await Promise.all([
      orgApi.projects(org),
      orgApi.campaigns(org),
      orgApi.publishers(org),
    ]);
    if (projs.ok) setProjects(projs.data);
    if (camps.ok) setCampaigns(camps.data);
    if (pubs.ok) setPublishers(pubs.data);
  }, []);

  useEffect(() => {
    if (orgId) void reload(orgId);
  }, [orgId, reload]);

  if (loading) return <div className="auth-wrap"><p style={{ color: "var(--muted)" }}>Loading…</p></div>;
  if (!me) return null;

  return (
    <main className="dash">
      <header className="page-head">
        <h1>Operations</h1>
        <p className="muted">Campaigns &amp; publishers · {selectedOrg(me.organizations)?.name ?? "No workspace"}</p>
      </header>

      {!orgId ? (
        <p className="muted">Create a workspace first.</p>
      ) : (
        <>
          <div className="tabs">
            <button className={tab === "campaigns" ? "tab on" : "tab"} onClick={() => setTab("campaigns")}>
              Campaigns ({campaigns.length})
            </button>
            <button className={tab === "publishers" ? "tab on" : "tab"} onClick={() => setTab("publishers")}>
              Publishers ({publishers.length})
            </button>
          </div>

          {tab === "campaigns" ? (
            <CampaignsTab
              org={orgId}
              projects={projects}
              campaigns={campaigns}
              onCreated={() => void reload(orgId)}
            />
          ) : (
            <PublishersTab org={orgId} publishers={publishers} onCreated={() => void reload(orgId)} />
          )}
        </>
      )}
    </main>
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

function CampaignsTab({
  org,
  projects,
  campaigns,
  onCreated,
}: {
  org: string;
  projects: ProjectView[];
  campaigns: CampaignView[];
  onCreated: () => void;
}) {
  const [projectId, setProjectId] = useState("");
  const [name, setName] = useState("");
  const [target, setTarget] = useState("");
  const [budget, setBudget] = useState("");
  const [currency, setCurrency] = useState("USD");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [rollups, setRollups] = useState<Record<string, CampaignRollup>>({});

  async function toggle(id: string) {
    if (rollups[id]) {
      setRollups((r) => {
        const c = { ...r };
        delete c[id];
        return c;
      });
      return;
    }
    const res = await orgApi.campaignRollup(org, id);
    if (res.ok) setRollups((r) => ({ ...r, [id]: res.data }));
  }

  const [editId, setEditId] = useState<string | null>(null);
  const [eName, setEName] = useState("");
  const [eTarget, setETarget] = useState("");
  const [eBudget, setEBudget] = useState("");
  const [eCurrency, setECurrency] = useState("USD");
  const [eBusy, setEBusy] = useState(false);

  function startEdit(c: CampaignView) {
    setEditId(c.id);
    setEName(c.name);
    setETarget(c.target_backlinks != null ? String(c.target_backlinks) : "");
    setEBudget(c.budget ?? "");
    setECurrency(c.currency ?? "USD");
    setError(null);
  }

  async function saveEdit(id: string) {
    const name = eName.trim();
    if (!name) {
      setError("Name is required.");
      return;
    }
    const body: { name: string; target_backlinks?: number; budget?: number; currency?: string } = { name };
    if (eTarget.trim()) body.target_backlinks = Number(eTarget);
    if (eBudget.trim()) {
      body.budget = Number(eBudget);
      body.currency = eCurrency.trim() || "USD";
    }
    setEBusy(true);
    const res = await orgApi.updateCampaign(org, id, body);
    setEBusy(false);
    if (!res.ok) {
      setError(res.detail);
      return;
    }
    setEditId(null);
    onCreated();
  }

  async function changeStatus(id: string, status: string) {
    const res = await orgApi.updateCampaign(org, id, { status });
    if (res.ok) onCreated();
    else setError(res.detail);
  }

  async function removeCampaign(id: string, cname: string) {
    if (!confirm(`Delete campaign "${cname}"? Its backlinks stay but are unassigned from it.`)) return;
    const res = await orgApi.deleteCampaign(org, id);
    if (res.ok) onCreated();
    else setError(res.detail);
  }

  async function create(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    if (!projectId) {
      setError("Pick a project.");
      return;
    }
    setBusy(true);
    const res = await orgApi.createCampaign(org, {
      project_id: projectId,
      name: name.trim(),
      target_backlinks: target ? Number(target) : undefined,
      budget: budget ? Number(budget) : undefined,
      currency: budget ? currency : undefined,
    });
    setBusy(false);
    if (!res.ok) {
      setError(res.detail);
      return;
    }
    setName("");
    setTarget("");
    setBudget("");
    onCreated();
  }

  return (
    <div className="ops-grid">
      <section className="panel">
        <div className="panel-head"><h2>Campaigns</h2></div>
        {campaigns.length === 0 ? (
          <p className="muted">No campaigns yet.</p>
        ) : (
          <ul className="ops-list">
            {campaigns.map((c) => (
              <li key={c.id}>
                {editId === c.id ? (
                  <div className="ops-edit">
                    <input value={eName} onChange={(e) => setEName(e.target.value)} placeholder="Name" />
                    <input
                      type="number"
                      min="1"
                      value={eTarget}
                      onChange={(e) => setETarget(e.target.value)}
                      placeholder="Target"
                      style={{ width: 90 }}
                    />
                    <input
                      type="number"
                      min="0"
                      step="0.01"
                      value={eBudget}
                      onChange={(e) => setEBudget(e.target.value)}
                      placeholder="Budget"
                      style={{ width: 110 }}
                    />
                    <input
                      value={eCurrency}
                      onChange={(e) => setECurrency(e.target.value.toUpperCase())}
                      maxLength={3}
                      placeholder="USD"
                      style={{ width: 60 }}
                    />
                    <button className="btn sm primary" disabled={eBusy} onClick={() => void saveEdit(c.id)}>
                      {eBusy ? "…" : "Save"}
                    </button>
                    <button className="btn sm" onClick={() => setEditId(null)}>Cancel</button>
                  </div>
                ) : (
                  <div className="ops-row-wrap">
                    <button className="ops-row" onClick={() => void toggle(c.id)}>
                      <span className="ops-name">{c.name}</span>
                      <span className="muted">{c.target_backlinks ? `target ${c.target_backlinks}` : "no target"}</span>
                    </button>
                    <div className="ops-actions">
                      <select
                        value={c.status}
                        title="Campaign status"
                        onChange={(e) => void changeStatus(c.id, e.target.value)}
                      >
                        {["draft", "active", "paused", "completed", "archived"].map((s) => (
                          <option key={s} value={s}>{s}</option>
                        ))}
                      </select>
                      <button className="btn sm" onClick={() => startEdit(c)}>Edit</button>
                      <button
                        className="btn sm"
                        style={{ color: "#b91c1c", borderColor: "#fecaca" }}
                        onClick={() => void removeCampaign(c.id, c.name)}
                      >
                        Delete
                      </button>
                    </div>
                  </div>
                )}
                {rollups[c.id] && <CampaignRollupCard r={rollups[c.id]} />}
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="panel">
        <div className="panel-head"><h2>New campaign</h2></div>
        <form onSubmit={create} className="ops-form">
          {error && <div className="auth-error" role="alert">{error}</div>}
          <label className="auth-field">
            <span>Project</span>
            <select value={projectId} onChange={(e) => setProjectId(e.target.value)} required>
              <option value="">Select…</option>
              {projects.map((p) => (
                <option key={p.id} value={p.id}>{p.name}, {p.domain}</option>
              ))}
            </select>
          </label>
          <label className="auth-field">
            <span>Name</span>
            <input value={name} onChange={(e) => setName(e.target.value)} required />
          </label>
          <label className="auth-field">
            <span>Target backlinks (optional)</span>
            <input type="number" min="1" value={target} onChange={(e) => setTarget(e.target.value)} />
          </label>
          <div className="ops-money">
            <label className="auth-field">
              <span>Budget (optional)</span>
              <input type="number" min="0" step="0.01" value={budget} onChange={(e) => setBudget(e.target.value)} />
            </label>
            <label className="auth-field">
              <span>Currency</span>
              <input value={currency} onChange={(e) => setCurrency(e.target.value.toUpperCase())} maxLength={3} />
            </label>
          </div>
          <button className="auth-btn" type="submit" disabled={busy}>{busy ? "Creating…" : "Create campaign"}</button>
        </form>
      </section>
    </div>
  );
}

function CampaignRollupCard({ r }: { r: CampaignRollup }) {
  return (
    <div className="rollup">
      <Stat label="Total" v={r.total} />
      <Stat label="Live" v={r.live} />
      <Stat label="Lost" v={r.lost} />
      <Stat label="Attention" v={r.needs_attention} />
      <Stat label="Removal" v={pct(r.removal_rate)} />
      <Stat label="Target" v={pct(r.target_progress)} />
      <Stat label="Spend" v={money(r.spend)} wide />
    </div>
  );
}

function PublishersTab({
  org,
  publishers,
  onCreated,
}: {
  org: string;
  publishers: PublisherView[];
  onCreated: () => void;
}) {
  const [name, setName] = useState("");
  const [domain, setDomain] = useState("");
  const [contactName, setContactName] = useState("");
  const [contactEmail, setContactEmail] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [rollups, setRollups] = useState<Record<string, PublisherRollup>>({});

  async function toggle(id: string) {
    if (rollups[id]) {
      setRollups((r) => {
        const c = { ...r };
        delete c[id];
        return c;
      });
      return;
    }
    const res = await orgApi.publisherRollup(org, id);
    if (res.ok) setRollups((r) => ({ ...r, [id]: res.data }));
  }

  async function create(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    const res = await orgApi.createPublisher(org, {
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
    onCreated();
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
          <button className="auth-btn" type="submit" disabled={busy}>{busy ? "Creating…" : "Create publisher"}</button>
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
