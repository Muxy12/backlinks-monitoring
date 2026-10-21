"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import {
  api,
  orgApi,
  type CampaignView,
  type MeResponse,
  type ProjectView,
} from "@/lib/api";
import { selectedOrg, selectedOrgId } from "@/lib/workspace";
import "@/app/auth.css";
import "../dashboard/dashboard.css";
import "../operations/operations.css";

// Link-building campaigns (BETA) index: a list of campaigns, each opening the
// goal-driven workflow (target pages, competitor gap, prospect pipeline).
export default function CampaignsPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [projects, setProjects] = useState<ProjectView[]>([]);
  const [campaigns, setCampaigns] = useState<CampaignView[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.me().then((res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      // The BETA is env-gated; if it is off for this workspace, there is nothing
      // here, send the user back to the dashboard.
      if (!res.data.features?.campaigns_beta) {
        router.replace("/dashboard");
        return;
      }
      setMe(res.data);
      setOrgId(selectedOrgId(res.data.organizations));
      setLoading(false);
    });
  }, [router]);

  const reload = useCallback(async (org: string) => {
    const [projs, camps] = await Promise.all([orgApi.projects(org), orgApi.campaigns(org)]);
    if (projs.ok) setProjects(projs.data);
    if (camps.ok) setCampaigns(camps.data);
  }, []);

  useEffect(() => {
    if (orgId) void reload(orgId);
  }, [orgId, reload]);

  if (loading)
    return (
      <div className="auth-wrap">
        <p style={{ color: "var(--muted)" }}>Loading…</p>
      </div>
    );
  if (!me) return null;

  return (
    <main className="dash">
      <header className="page-head">
        <h1>
          Link Building{" "}
          <span
            style={{
              fontSize: 11,
              fontWeight: 700,
              padding: "2px 7px",
              borderRadius: 999,
              background: "#2f6bff",
              color: "#fff",
              verticalAlign: "middle",
            }}
          >
            BETA
          </span>
        </h1>
        <p className="muted">
          Plan what to do next to grow your backlinks · {selectedOrg(me.organizations)?.name ?? "No workspace"}
        </p>
      </header>

      {!orgId ? (
        <p className="muted">Create a workspace first.</p>
      ) : (
        <div className="ops-grid">
          <section className="panel">
            <div className="panel-head">
              <h2>Campaigns</h2>
            </div>
            {campaigns.length === 0 ? (
              <p className="muted">No campaigns yet. Create one to start building links.</p>
            ) : (
              <ul className="ops-list">
                {campaigns.map((c) => (
                  <li key={c.id}>
                    <button
                      className="ops-row"
                      onClick={() => router.push(`/campaigns/${c.id}`)}
                    >
                      <span className="ops-name">
                        {c.name}
                        {c.system_kind === "reclamation" && (
                          <span
                            className="badge"
                            style={{
                              marginLeft: 8,
                              background: "#e0f2fe",
                              color: "#075985",
                            }}
                          >
                            ♻ Auto
                          </span>
                        )}
                      </span>
                      <span className="muted">
                        {c.status}
                        {c.objective ? ` · ${c.objective}` : ""}
                      </span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </section>

          <NewCampaignForm org={orgId} projects={projects} onCreated={() => void reload(orgId)} />
        </div>
      )}
    </main>
  );
}

function NewCampaignForm({
  org,
  projects,
  onCreated,
}: {
  org: string;
  projects: ProjectView[];
  onCreated: () => void;
}) {
  const router = useRouter();
  const [projectId, setProjectId] = useState("");
  const [name, setName] = useState("");
  const [objective, setObjective] = useState("");
  const [targetUrl, setTargetUrl] = useState("");
  const [target, setTarget] = useState("");
  const [budget, setBudget] = useState("");
  const [currency, setCurrency] = useState("USD");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

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
      objective: objective.trim() || undefined,
      primary_target_url: targetUrl.trim() || undefined,
      target_backlinks: target ? Number(target) : undefined,
      budget: budget ? Number(budget) : undefined,
      currency: budget ? currency : undefined,
    });
    setBusy(false);
    if (!res.ok) {
      setError(res.detail);
      return;
    }
    router.push(`/campaigns/${res.data.id}`);
    onCreated();
  }

  return (
    <section className="panel">
      <div className="panel-head">
        <h2>New campaign</h2>
      </div>
      <form onSubmit={create} className="ops-form">
        {error && <div className="auth-error">{error}</div>}
        <label className="auth-field">
          <span>Project</span>
          <select value={projectId} onChange={(e) => setProjectId(e.target.value)} required>
            <option value="">Select…</option>
            {projects.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}, {p.domain}
              </option>
            ))}
          </select>
        </label>
        <label className="auth-field">
          <span>Name</span>
          <input value={name} onChange={(e) => setName(e.target.value)} required />
        </label>
        <label className="auth-field">
          <span>Objective (optional)</span>
          <input
            value={objective}
            onChange={(e) => setObjective(e.target.value)}
            placeholder="Grow topical authority for /guide"
          />
        </label>
        <label className="auth-field">
          <span>Primary target page (optional)</span>
          <input
            value={targetUrl}
            onChange={(e) => setTargetUrl(e.target.value)}
            placeholder="https://example.com/guide"
          />
        </label>
        <label className="auth-field">
          <span>Target backlinks (optional)</span>
          <input type="number" min="1" value={target} onChange={(e) => setTarget(e.target.value)} />
        </label>
        <div className="ops-money">
          <label className="auth-field">
            <span>Budget (optional)</span>
            <input
              type="number"
              min="0"
              step="0.01"
              value={budget}
              onChange={(e) => setBudget(e.target.value)}
            />
          </label>
          <label className="auth-field">
            <span>Currency</span>
            <input
              value={currency}
              onChange={(e) => setCurrency(e.target.value.toUpperCase())}
              maxLength={3}
            />
          </label>
        </div>
        <button className="auth-btn" type="submit" disabled={busy}>
          {busy ? "Creating…" : "Create campaign"}
        </button>
      </form>
    </section>
  );
}
