"use client";

import { useCallback, useEffect, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import {
  api,
  orgApi,
  type CampaignImpact,
  type CampaignSummary,
  type CampaignTargetView,
  type CampaignView,
  type MeResponse,
  type OutreachStatusView,
  type ProspectView,
} from "@/lib/api";
import { selectedOrgId } from "@/lib/workspace";
import {
  PIPELINE_STAGES,
  STAGES,
  STAGE_BADGE,
  isReclaim,
  sourceLabel,
  stageLabel,
} from "@/lib/campaigns";
import "@/app/auth.css";
import "../../dashboard/dashboard.css";
import "../../operations/operations.css";
import "./campaigns.css";

type Tab = "overview" | "gap" | "pipeline";
type GapRow = { source_domain: string; competitor_count: number };

function money(rows: { currency: string; amount: string }[]) {
  return rows.length === 0 ? "—" : rows.map((s) => `${s.amount} ${s.currency}`).join(" · ");
}

export default function CampaignDetailPage() {
  const router = useRouter();
  const params = useParams<{ id: string }>();
  const campaignId = params.id;
  const [orgId, setOrgId] = useState<string | null>(null);
  const [me, setMe] = useState<MeResponse | null>(null);
  const [campaign, setCampaign] = useState<CampaignView | null>(null);
  const [summary, setSummary] = useState<CampaignSummary | null>(null);
  const [targets, setTargets] = useState<CampaignTargetView[]>([]);
  const [prospects, setProspects] = useState<ProspectView[]>([]);
  const [tab, setTab] = useState<Tab>("overview");
  const [loading, setLoading] = useState(true);
  const [notFound, setNotFound] = useState(false);
  const [editing, setEditing] = useState(false);
  const [deleting, setDeleting] = useState(false);

  async function removeCampaign() {
    if (!orgId || !campaign) return;
    if (!confirm(`Delete campaign “${campaign.name}”? This removes its targets and prospects. This cannot be undone.`)) {
      return;
    }
    setDeleting(true);
    const res = await orgApi.deleteCampaign(orgId, campaign.id);
    setDeleting(false);
    if (res.ok) router.push("/campaigns");
    else alert(res.detail);
  }

  useEffect(() => {
    api.me().then((res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      if (!res.data.features?.campaigns_beta) {
        router.replace("/dashboard");
        return;
      }
      setMe(res.data);
      setOrgId(selectedOrgId(res.data.organizations));
    });
  }, [router]);

  const reloadSummary = useCallback(
    async (org: string) => {
      const s = await orgApi.campaignSummary(org, campaignId);
      if (s.ok) setSummary(s.data);
    },
    [campaignId],
  );
  const reloadTargets = useCallback(
    async (org: string) => {
      const t = await orgApi.campaignTargets(org, campaignId);
      if (t.ok) setTargets(t.data);
    },
    [campaignId],
  );
  const reloadProspects = useCallback(
    async (org: string) => {
      const p = await orgApi.prospects(org, campaignId);
      if (p.ok) setProspects(p.data);
    },
    [campaignId],
  );

  useEffect(() => {
    if (!orgId) return;
    (async () => {
      const c = await orgApi.campaign(orgId, campaignId);
      if (!c.ok) {
        setNotFound(true);
        setLoading(false);
        return;
      }
      setCampaign(c.data);
      await Promise.all([reloadSummary(orgId), reloadTargets(orgId), reloadProspects(orgId)]);
      setLoading(false);
    })();
  }, [orgId, campaignId, reloadSummary, reloadTargets, reloadProspects]);

  if (loading)
    return (
      <div className="auth-wrap">
        <p style={{ color: "var(--muted)" }}>Loading…</p>
      </div>
    );
  if (notFound || !campaign || !orgId)
    return (
      <main className="dash">
        <p className="muted">Campaign not found.</p>
        <button className="btn sm" onClick={() => router.push("/campaigns")}>
          ← Back to campaigns
        </button>
      </main>
    );

  const senderName = me?.user.name || undefined;

  return (
    <main className="dash">
      <header className="page-head with-actions">
        <div>
          <h1>{campaign.name}</h1>
          <p className="muted">
            <a href="/campaigns" style={{ color: "var(--muted)" }}>
              Link Building
            </a>{" "}
            · {campaign.status}
            {campaign.objective ? ` · ${campaign.objective}` : ""}
          </p>
        </div>
        <div className="ph-actions">
          <button className="btn sm" onClick={() => setEditing(true)}>
            Edit
          </button>
          <button className="btn sm danger" disabled={deleting} onClick={() => void removeCampaign()}>
            {deleting ? "Deleting…" : "Delete"}
          </button>
        </div>
      </header>

      {editing && (
        <EditCampaignPanel
          org={orgId}
          campaign={campaign}
          onSaved={(c) => {
            setCampaign(c);
            setEditing(false);
          }}
          onCancel={() => setEditing(false)}
        />
      )}

      <div className="tabs">
        {(["overview", "gap", "pipeline"] as Tab[]).map((t) => (
          <button key={t} className={tab === t ? "tab on" : "tab"} onClick={() => setTab(t)}>
            {t === "overview" ? "Overview" : t === "gap" ? "Gap" : `Pipeline (${prospects.length})`}
          </button>
        ))}
      </div>

      {tab === "overview" && (
        <OverviewTab
          org={orgId}
          campaign={campaign}
          summary={summary}
          targets={targets}
          gsc={me?.features?.gsc ?? false}
          onCampaign={setCampaign}
          onTargets={() => void reloadTargets(orgId)}
        />
      )}
      {tab === "gap" && (
        <GapTab
          org={orgId}
          campaign={campaign}
          onSeeded={async () => {
            await Promise.all([reloadProspects(orgId), reloadSummary(orgId)]);
            setTab("pipeline");
          }}
        />
      )}
      {tab === "pipeline" && (
        <PipelineTab
          org={orgId}
          campaignId={campaignId}
          prospects={prospects}
          senderName={senderName}
          onChanged={() => Promise.all([reloadProspects(orgId), reloadSummary(orgId)])}
        />
      )}
    </main>
  );
}

// ─── Overview ──────────────────────────────────────────────────────────────
function OverviewTab({
  org,
  campaign,
  summary,
  targets,
  gsc,
  onCampaign,
  onTargets,
}: {
  org: string;
  campaign: CampaignView;
  summary: CampaignSummary | null;
  targets: CampaignTargetView[];
  gsc: boolean;
  onCampaign: (c: CampaignView) => void;
  onTargets: () => void;
}) {
  const [url, setUrl] = useState("");
  const [label, setLabel] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const [impact, setImpact] = useState<CampaignImpact | null>(null);

  useEffect(() => {
    if (!gsc) return;
    let live = true;
    void orgApi.campaignImpact(org, campaign.id).then((r) => {
      if (live && r.ok) setImpact(r.data);
    });
    return () => {
      live = false;
    };
  }, [gsc, org, campaign.id]);

  async function addTarget(e: React.FormEvent) {
    e.preventDefault();
    setErr(null);
    const res = await orgApi.createCampaignTarget(org, campaign.id, {
      url: url.trim(),
      label: label.trim() || undefined,
    });
    if (!res.ok) {
      setErr(res.detail);
      return;
    }
    setUrl("");
    setLabel("");
    onTargets();
  }

  async function changeStatus(status: string) {
    const res = await orgApi.updateCampaign(org, campaign.id, { status });
    if (res.ok) onCampaign(res.data);
  }

  const progress = summary?.target_progress;
  return (
    <>
      <div className="tiles">
        <Tile label="Prospects" value={summary ? summary.prospects_total : "—"} />
        <Tile label="Backlinks" value={summary ? summary.total : "—"} />
        <Tile label="Live" value={summary ? summary.live : "—"} />
        <Tile
          label="Target progress"
          value={progress == null ? "—" : `${Math.round(progress * 100)}%`}
          hint={campaign.target_backlinks ? `of ${campaign.target_backlinks}` : "no target"}
        />
        <Tile
          label="Budget"
          value={campaign.budget ? `${campaign.budget} ${campaign.currency ?? ""}` : "—"}
          hint={summary && summary.spend.length ? `spent ${money(summary.spend)}` : "no spend yet"}
        />
      </div>

      <div className="ops-grid" style={{ marginTop: 16 }}>
        <section className="panel">
          <div className="panel-head">
            <h2>Campaign</h2>
          </div>
          <div className="detail-grid">
            <span>Status</span>
            <span>
              <select value={campaign.status} onChange={(e) => void changeStatus(e.target.value)}>
                {["draft", "active", "paused", "completed", "archived"].map((s) => (
                  <option key={s} value={s}>
                    {s}
                  </option>
                ))}
              </select>
            </span>
            <span>Objective</span>
            <span>{campaign.objective || "—"}</span>
            <span>Primary target</span>
            <span className="mono">{campaign.primary_target_url || "—"}</span>
            <span>Dates</span>
            <span>
              {campaign.start_date || "—"} → {campaign.end_date || "—"}
            </span>
          </div>
        </section>

        <section className="panel">
          <div className="panel-head">
            <h2>Target pages</h2>
          </div>
          {targets.length === 0 ? (
            <p className="muted">No target pages yet.</p>
          ) : (
            <ul className="ops-list">
              {targets.map((t) => (
                <li key={t.id}>
                  <div className="ops-row-wrap">
                    <span className="ops-row" style={{ cursor: "default" }}>
                      <span className="mono">{t.url}</span>
                      {t.label && <span className="muted">{t.label}</span>}
                    </span>
                    <div className="ops-actions">
                      <button
                        className="btn sm"
                        onClick={async () => {
                          await orgApi.deleteCampaignTarget(org, t.id);
                          onTargets();
                        }}
                      >
                        Remove
                      </button>
                    </div>
                  </div>
                </li>
              ))}
            </ul>
          )}
          <form onSubmit={addTarget} className="ops-form" style={{ marginTop: 10 }}>
            {err && <div className="auth-error">{err}</div>}
            <label className="auth-field">
              <span>Target URL</span>
              <input
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                placeholder="https://example.com/page"
                required
              />
            </label>
            <label className="auth-field">
              <span>Label (optional)</span>
              <input value={label} onChange={(e) => setLabel(e.target.value)} />
            </label>
            <button className="btn sm primary" type="submit">
              Add target page
            </button>
          </form>
        </section>

        {gsc && (
          <section className="panel">
            <div className="panel-head">
              <h2>
                Post-Link Impact{" "}
                <span
                  style={{
                    fontSize: 9,
                    fontWeight: 700,
                    letterSpacing: "0.08em",
                    padding: "2px 6px",
                    borderRadius: 999,
                    background: "var(--grad-soft)",
                    color: "var(--violet)",
                    verticalAlign: "middle",
                  }}
                >
                  BETA
                </span>
              </h2>
            </div>
            {!impact ? (
              <p className="muted">Loading impact…</p>
            ) : impact.eligible_links === 0 ? (
              <p className="muted">{impact.summary}</p>
            ) : (
              <>
                <p className="muted" style={{ marginTop: 0 }}>
                  {impact.summary}
                </p>
                <div className="detail-grid">
                  <span>Tracked links</span>
                  <span>{impact.eligible_links}</span>
                  <span>With 28-day data</span>
                  <span>{impact.measured_links}</span>
                  <span>Median visibility Δ</span>
                  <span className="mono">
                    {impact.median_visibility_change_percent == null
                      ? "—"
                      : `${impact.median_visibility_change_percent >= 0 ? "+" : ""}${Math.round(
                          impact.median_visibility_change_percent,
                        )}%`}
                  </span>
                  <span>Median position Δ</span>
                  <span className="mono">
                    {impact.median_position_delta == null
                      ? "—"
                      : `${impact.median_position_delta > 0 ? "+" : ""}${impact.median_position_delta.toFixed(
                          1,
                        )}`}
                    {impact.median_position_delta != null && (
                      <span className="muted" style={{ marginLeft: 6, fontSize: 11 }}>
                        (lower = better)
                      </span>
                    )}
                  </span>
                </div>
                <div
                  style={{ display: "flex", flexWrap: "wrap", gap: 8, margin: "10px 0" }}
                >
                  {(
                    [
                      ["var(--live)", impact.positive, "up"],
                      ["var(--changed)", impact.neutral, "flat"],
                      ["var(--lost)", impact.negative, "down"],
                      ["var(--pending)", impact.pending, "pending"],
                    ] as const
                  ).map(([color, n, label]) => (
                    <span
                      key={label}
                      style={{
                        display: "inline-flex",
                        alignItems: "center",
                        gap: 6,
                        fontSize: 12,
                        padding: "3px 9px",
                        borderRadius: 999,
                        border: "1px solid var(--border)",
                      }}
                    >
                      <span
                        style={{
                          width: 7,
                          height: 7,
                          borderRadius: "50%",
                          background: color as string,
                        }}
                      />
                      {n} {label}
                    </span>
                  ))}
                </div>
                <p
                  className="muted"
                  style={{ fontSize: 11.5, fontStyle: "italic", margin: 0 }}
                >
                  Observed correlation, not proof of causation.
                </p>
              </>
            )}
          </section>
        )}
      </div>
    </>
  );
}

// ─── Gap ───────────────────────────────────────────────────────────────────
function GapTab({
  org,
  campaign,
  onSeeded,
}: {
  org: string;
  campaign: CampaignView;
  onSeeded: () => void;
}) {
  const [rows, setRows] = useState<GapRow[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  const load = useCallback(async () => {
    const res = await orgApi.backlinkGap(org, campaign.project_id, 1);
    setRows(res.ok ? res.data : []);
  }, [org, campaign.project_id]);

  useEffect(() => {
    void load();
  }, [load]);

  async function seedAll() {
    setBusy(true);
    setMsg(null);
    const res = await orgApi.seedProspectsFromGap(org, campaign.id, 1);
    setBusy(false);
    if (res.ok) {
      setMsg(`Added ${res.data.created} prospect${res.data.created === 1 ? "" : "s"}.`);
      onSeeded();
    } else {
      setMsg(res.detail);
    }
  }

  async function addOne(domain: string) {
    await orgApi.createProspect(org, campaign.id, { domain });
    setMsg(`Added ${domain}.`);
  }

  return (
    <section className="panel">
      <div className="panel-head">
        <h2>Competitor gap</h2>
        <div className="panel-actions">
          <button className="btn sm primary" onClick={() => void seedAll()} disabled={busy || !rows?.length}>
            {busy ? "Seeding…" : "Seed all as prospects"}
          </button>
        </div>
      </div>
      {msg && <p className="muted">{msg}</p>}
      {rows == null ? (
        <p className="muted">Loading…</p>
      ) : rows.length === 0 ? (
        <p className="muted">
          No gap found. Add competitors and import their backlinks (Competitors page) to surface
          domains that link to them but not to you.
        </p>
      ) : (
        <div className="bl-scroll">
          <table className="bl-table">
            <thead>
              <tr>
                <th>Referring domain</th>
                <th>Competitors linking</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <tr key={r.source_domain}>
                  <td className="mono">{r.source_domain}</td>
                  <td>{r.competitor_count}</td>
                  <td>
                    <button className="btn sm" onClick={() => void addOne(r.source_domain)}>
                      Add
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}

// ─── Pipeline ──────────────────────────────────────────────────────────────
function PipelineTab({
  org,
  campaignId,
  prospects,
  senderName,
  onChanged,
}: {
  org: string;
  campaignId: string;
  prospects: ProspectView[];
  senderName?: string;
  onChanged: () => void | Promise<unknown>;
}) {
  const [open, setOpen] = useState<ProspectView | null>(null);
  const [sel, setSel] = useState<Set<string>>(new Set());
  const [bulkStage, setBulkStage] = useState("qualified");

  const [addDomain, setAddDomain] = useState("");

  function toggle(id: string) {
    setSel((s) => {
      const n = new Set(s);
      if (n.has(id)) n.delete(id);
      else n.add(id);
      return n;
    });
  }

  async function applyBulk() {
    if (sel.size === 0) return;
    await orgApi.bulkProspectStage(org, campaignId, [...sel], bulkStage);
    setSel(new Set());
    await onChanged();
  }

  async function addManual(e: React.FormEvent) {
    e.preventDefault();
    if (!addDomain.trim()) return;
    await orgApi.createProspect(org, campaignId, { domain: addDomain.trim() });
    setAddDomain("");
    await onChanged();
  }

  const stagesToShow = [...PIPELINE_STAGES, "lost"];
  const byStage = (stage: string) => prospects.filter((p) => p.stage === stage);

  return (
    <>
      <section className="panel">
        <div className="panel-head">
          <h2>Pipeline</h2>
          <div className="panel-actions">
            <form onSubmit={addManual} style={{ display: "flex", gap: 6 }}>
              <input
                value={addDomain}
                onChange={(e) => setAddDomain(e.target.value)}
                placeholder="add a domain…"
                style={{ width: 170 }}
              />
              <button className="btn sm" type="submit">
                Add prospect
              </button>
            </form>
          </div>
        </div>

        {sel.size > 0 && (
          <div className="bulk-bar">
            <span>{sel.size} selected</span>
            <select value={bulkStage} onChange={(e) => setBulkStage(e.target.value)}>
              {STAGES.map(([slug, lbl]) => (
                <option key={slug} value={slug}>
                  {lbl}
                </option>
              ))}
            </select>
            <button className="btn sm primary" onClick={() => void applyBulk()}>
              Move to stage
            </button>
            <button className="btn sm" onClick={() => setSel(new Set())}>
              Clear
            </button>
          </div>
        )}

        {prospects.length === 0 ? (
          <p className="muted">
            No prospects yet. Seed them from the Gap tab, or add a domain above.
          </p>
        ) : (
          stagesToShow.map((stage) => {
            const rows = byStage(stage);
            if (rows.length === 0) return null;
            const badge = STAGE_BADGE[stage];
            return (
              <div key={stage} className="stage-group">
                <div className="stage-head">
                  <span
                    className="badge"
                    style={{ background: badge?.bg, color: badge?.fg }}
                  >
                    {stageLabel(stage)}
                  </span>
                  <span className="muted">{rows.length}</span>
                </div>
                <div className="bl-scroll">
                  <table className="bl-table">
                    <tbody>
                      {rows.map((p) => (
                        <tr key={p.id}>
                          <td style={{ width: 28 }}>
                            <input
                              type="checkbox"
                              checked={sel.has(p.id)}
                              onChange={() => toggle(p.id)}
                              aria-label={`Select ${p.domain}`}
                            />
                          </td>
                          <td className="mono">{p.domain}</td>
                          <td>
                            <span className="muted">{sourceLabel(p.source)}</span>
                          </td>
                          <td>{p.score ?? "—"}</td>
                          <td style={{ textAlign: "right" }}>
                            <button className="btn sm" onClick={() => setOpen(p)}>
                              Open
                            </button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            );
          })
        )}
      </section>

      {open && (
        <ProspectDrawer
          org={org}
          prospect={open}
          senderName={senderName}
          onClose={() => setOpen(null)}
          onChanged={async () => {
            await onChanged();
          }}
        />
      )}
    </>
  );
}

// ─── Prospect drawer ─────────────────────────────────────────────────────────
function ProspectDrawer({
  org,
  prospect,
  senderName,
  onClose,
  onChanged,
}: {
  org: string;
  prospect: ProspectView;
  senderName?: string;
  onClose: () => void;
  onChanged: () => Promise<void>;
}) {
  const [stage, setStage] = useState(prospect.stage);
  const [contactName, setContactName] = useState(prospect.contact_name ?? "");
  const [contactEmail, setContactEmail] = useState(prospect.contact_email ?? "");
  const [targetUrl, setTargetUrl] = useState(prospect.target_url ?? "");
  const [cost, setCost] = useState(prospect.estimated_cost ?? "");
  const [currency, setCurrency] = useState(prospect.currency ?? "USD");
  const [notes, setNotes] = useState(prospect.notes ?? "");
  const [msg, setMsg] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const [draft, setDraft] = useState<{ subject: string; body: string } | null>(null);
  const [copied, setCopied] = useState(false);

  const [srcUrl, setSrcUrl] = useState("");
  const [converted, setConverted] = useState<string | null>(prospect.won_backlink_id ?? null);

  const [outreach, setOutreach] = useState<OutreachStatusView | null>(null);
  useEffect(() => {
    void orgApi.outreachStatus(org, prospect.id).then((r) => {
      if (r.ok) setOutreach(r.data);
    });
  }, [org, prospect.id]);

  async function startSequence() {
    setBusy(true);
    setMsg(null);
    const res = await orgApi.startOutreach(org, prospect.id);
    setBusy(false);
    if (!res.ok) {
      setMsg(res.detail);
      return;
    }
    setMsg("Sequence started.");
    const r = await orgApi.outreachStatus(org, prospect.id);
    if (r.ok) setOutreach(r.data);
    await onChanged();
  }

  async function save() {
    setBusy(true);
    setMsg(null);
    const res = await orgApi.updateProspect(org, prospect.id, {
      stage,
      contact_name: contactName.trim() || undefined,
      contact_email: contactEmail.trim() || undefined,
      target_url: targetUrl.trim() || undefined,
      estimated_cost: cost.trim() ? Number(cost) : undefined,
      currency: cost.trim() ? currency : undefined,
      notes: notes.trim() || undefined,
    });
    setBusy(false);
    if (!res.ok) {
      setMsg(res.detail);
      return;
    }
    setMsg("Saved.");
    await onChanged();
  }

  async function genDraft() {
    const res = await orgApi.prospectDraft(org, prospect.id, senderName);
    if (res.ok) {
      setDraft(res.data);
      setCopied(false);
    }
  }

  async function copyDraft() {
    if (!draft) return;
    try {
      await navigator.clipboard.writeText(`Subject: ${draft.subject}\n\n${draft.body}`);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  }

  async function convert() {
    if (!srcUrl.trim()) {
      setMsg("Enter the URL where the link was placed.");
      return;
    }
    setBusy(true);
    setMsg(null);
    const res = await orgApi.convertProspect(org, prospect.id, {
      source_url: srcUrl.trim(),
      target_url: targetUrl.trim() || undefined,
      cost: cost.trim() ? Number(cost) : undefined,
      currency: cost.trim() ? currency : undefined,
    });
    setBusy(false);
    if (!res.ok) {
      setMsg(res.detail);
      return;
    }
    setConverted(res.data.backlink_id);
    setMsg("Converted to a protected backlink.");
    await onChanged();
  }

  return (
    <>
      <div className="drawer-backdrop" onClick={onClose} />
      <aside className="drawer" role="dialog" aria-label="Prospect">
        <div className="panel-head">
          <h2 className="mono">{prospect.domain}</h2>
          <button className="btn sm" onClick={onClose}>
            Close
          </button>
        </div>

        <div className="drawer-sub">Details</div>
        <div className="detail-grid">
          <span>Source</span>
          <span>{sourceLabel(prospect.source)}</span>
          <span>Score</span>
          <span>
            {prospect.score ?? "—"}
            {prospect.score_reason ? ` · ${prospect.score_reason}` : ""}
          </span>
          {prospect.competitor_count != null && (
            <>
              <span>Competitors linking</span>
              <span>{prospect.competitor_count}</span>
            </>
          )}
        </div>

        {isReclaim(prospect.source) && (
          <div
            className="reclaim-note"
            style={{
              margin: "12px 0",
              padding: "12px 14px",
              border: "1px solid var(--line)",
              borderRadius: 10,
              background: "var(--bg-subtle, rgba(14,165,233,.06))",
            }}
          >
            <span
              className="badge"
              style={{ background: "#e0f2fe", color: "#075985" }}
            >
              ♻ Reclamation
            </span>
            {prospect.notes && (
              <p className="muted" style={{ margin: "8px 0 0" }}>
                {prospect.notes}
              </p>
            )}
            {prospect.stage === "won" && prospect.won_backlink_id && (
              <p style={{ margin: "8px 0 0", color: "#166534", fontWeight: 600 }}>
                ✓ This link is live again — auto-won by monitoring.
              </p>
            )}
          </div>
        )}

        <div className="drawer-sub">Edit</div>
        {msg && <p className="muted">{msg}</p>}
        <label className="auth-field">
          <span>Stage</span>
          <select value={stage} onChange={(e) => setStage(e.target.value)}>
            {STAGES.map(([slug, lbl]) => (
              <option key={slug} value={slug}>
                {lbl}
              </option>
            ))}
          </select>
        </label>
        <label className="auth-field">
          <span>Contact name</span>
          <input value={contactName} onChange={(e) => setContactName(e.target.value)} />
        </label>
        <label className="auth-field">
          <span>Contact email</span>
          <input value={contactEmail} onChange={(e) => setContactEmail(e.target.value)} />
        </label>
        <label className="auth-field">
          <span>Target page</span>
          <input
            value={targetUrl}
            onChange={(e) => setTargetUrl(e.target.value)}
            placeholder="https://example.com/page"
          />
        </label>
        <div className="ops-money">
          <label className="auth-field">
            <span>Est. cost</span>
            <input
              type="number"
              min="0"
              step="0.01"
              value={cost}
              onChange={(e) => setCost(e.target.value)}
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
        <label className="auth-field">
          <span>Notes</span>
          <textarea value={notes} onChange={(e) => setNotes(e.target.value)} rows={2} />
        </label>
        <button className="btn sm primary" onClick={() => void save()} disabled={busy}>
          {busy ? "Saving…" : "Save"}
        </button>

        <div className="drawer-sub">Outreach draft</div>
        <button className="btn sm" onClick={() => void genDraft()}>
          Generate draft
        </button>
        {draft && (
          <div style={{ marginTop: 8 }}>
            <input readOnly value={draft.subject} style={{ width: "100%", marginBottom: 6 }} />
            <textarea readOnly value={draft.body} rows={8} style={{ width: "100%" }} />
            <button className="btn sm" onClick={() => void copyDraft()} style={{ marginTop: 6 }}>
              {copied ? "Copied!" : "Copy"}
            </button>
          </div>
        )}

        <div className="drawer-sub">Outreach sequence</div>
        {outreach && outreach.messages.length > 0 ? (
          <ul style={{ listStyle: "none", padding: 0, margin: 0, fontSize: ".9rem" }}>
            {outreach.messages.map((m) => {
              const when = m.status === "sent" && m.sent_at ? m.sent_at : m.scheduled_at;
              const verb = m.status === "sent" ? "sent" : m.status === "queued" ? "due" : m.status;
              return (
                <li
                  key={m.step}
                  style={{ display: "flex", justifyContent: "space-between", padding: "4px 0" }}
                >
                  <span>Step {m.step}</span>
                  <span className="muted">
                    {verb} · {new Date(when).toLocaleDateString()}
                  </span>
                </li>
              );
            })}
          </ul>
        ) : (
          <>
            <p className="muted" style={{ margin: "0 0 8px" }}>
              Send an automatic 3-step follow-up (day 0, +3, +7) from LinkPulz. Replies stop it
              automatically; every email has an unsubscribe link.
            </p>
            {outreach && !outreach.configured && (
              <p className="muted" style={{ margin: "0 0 8px", color: "#9a3412" }}>
                Automated sending isn’t enabled on this server yet.
              </p>
            )}
            <button
              className="btn sm"
              onClick={() => void startSequence()}
              disabled={busy || !outreach || !outreach.configured || !contactEmail.trim()}
            >
              Start sequence
            </button>
          </>
        )}

        <div className="drawer-sub">Convert to backlink</div>
        {converted ? (
          <p className="muted">
            Won — created a protected backlink.{" "}
            <a href={`/dashboard`}>View in dashboard</a>
          </p>
        ) : (
          <>
            <label className="auth-field">
              <span>Placed link URL</span>
              <input
                value={srcUrl}
                onChange={(e) => setSrcUrl(e.target.value)}
                placeholder="https://their-site.com/post"
              />
            </label>
            <p className="muted" style={{ fontSize: 12 }}>
              Marks the prospect won and creates a protected, monitored backlink for this campaign
              (target: {targetUrl || "campaign’s primary page"}).
            </p>
            <button className="btn sm primary" onClick={() => void convert()} disabled={busy}>
              {busy ? "Converting…" : "Convert to backlink"}
            </button>
          </>
        )}
      </aside>
    </>
  );
}

function Tile({
  label,
  value,
  hint,
}: {
  label: string;
  value: number | string;
  hint?: string;
}) {
  return (
    <div className="tile">
      <div className="tile-top">
        <span className="tile-label">{label}</span>
      </div>
      <div className="tile-value">{value}</div>
      {hint && <div className="tile-hint">{hint}</div>}
    </div>
  );
}

function EditCampaignPanel({
  org,
  campaign,
  onSaved,
  onCancel,
}: {
  org: string;
  campaign: CampaignView;
  onSaved: (c: CampaignView) => void;
  onCancel: () => void;
}) {
  const [name, setName] = useState(campaign.name);
  const [description, setDescription] = useState(campaign.description ?? "");
  const [status, setStatus] = useState(campaign.status);
  const [objective, setObjective] = useState(campaign.objective ?? "");
  const [targetUrl, setTargetUrl] = useState(campaign.primary_target_url ?? "");
  const [target, setTarget] = useState(campaign.target_backlinks?.toString() ?? "");
  const [budget, setBudget] = useState(campaign.budget ?? "");
  const [currency, setCurrency] = useState(campaign.currency ?? "USD");
  const [startDate, setStartDate] = useState(campaign.start_date ?? "");
  const [endDate, setEndDate] = useState(campaign.end_date ?? "");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function save(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    const res = await orgApi.updateCampaign(org, campaign.id, {
      name: name.trim(),
      description: description.trim(),
      status,
      objective: objective.trim(),
      primary_target_url: targetUrl.trim(),
      target_backlinks: target ? Number(target) : undefined,
      budget: budget ? Number(budget) : undefined,
      currency: budget ? currency : undefined,
      start_date: startDate || undefined,
      end_date: endDate || undefined,
    });
    setBusy(false);
    if (!res.ok) {
      setError(res.detail);
      return;
    }
    onSaved(res.data);
  }

  return (
    <section className="panel" style={{ marginBottom: 16 }}>
      <div className="panel-head">
        <h2>Edit campaign</h2>
      </div>
      <form onSubmit={save} className="ops-form">
        {error && <div className="auth-error">{error}</div>}
        <label className="auth-field">
          <span>Name</span>
          <input value={name} onChange={(e) => setName(e.target.value)} required />
        </label>
        <label className="auth-field">
          <span>Status</span>
          <select value={status} onChange={(e) => setStatus(e.target.value)}>
            {["draft", "active", "paused", "completed", "archived"].map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </select>
        </label>
        <label className="auth-field">
          <span>Objective</span>
          <input value={objective} onChange={(e) => setObjective(e.target.value)} placeholder="Grow topical authority for /guide" />
        </label>
        <label className="auth-field">
          <span>Description</span>
          <textarea value={description} onChange={(e) => setDescription(e.target.value)} rows={2} />
        </label>
        <label className="auth-field">
          <span>Primary target page</span>
          <input value={targetUrl} onChange={(e) => setTargetUrl(e.target.value)} placeholder="https://example.com/guide" />
        </label>
        <label className="auth-field">
          <span>Target backlinks</span>
          <input type="number" min="1" value={target} onChange={(e) => setTarget(e.target.value)} />
        </label>
        <div className="ops-money">
          <label className="auth-field">
            <span>Budget</span>
            <input type="number" min="0" step="0.01" value={budget} onChange={(e) => setBudget(e.target.value)} />
          </label>
          <label className="auth-field">
            <span>Currency</span>
            <input value={currency} onChange={(e) => setCurrency(e.target.value.toUpperCase())} maxLength={3} />
          </label>
        </div>
        <div className="ops-money">
          <label className="auth-field">
            <span>Start date</span>
            <input type="date" value={startDate} onChange={(e) => setStartDate(e.target.value)} />
          </label>
          <label className="auth-field">
            <span>End date</span>
            <input type="date" value={endDate} onChange={(e) => setEndDate(e.target.value)} />
          </label>
        </div>
        <div style={{ display: "flex", gap: 10 }}>
          <button className="auth-btn" type="submit" disabled={busy}>
            {busy ? "Saving…" : "Save changes"}
          </button>
          <button className="btn" type="button" disabled={busy} onClick={onCancel}>
            Cancel
          </button>
        </div>
      </form>
    </section>
  );
}
