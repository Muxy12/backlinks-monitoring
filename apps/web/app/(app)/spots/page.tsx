"use client";
import { PageLoading } from "@/components/PageLoading";

import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import {
  api,
  orgApi,
  type CampaignView,
  type MeResponse,
  type ProjectView,
  type SpotKeywordsResponse,
  type SpotKeywordView,
  type SpotsResponse,
} from "@/lib/api";
import { selectedOrgId } from "@/lib/workspace";
import "@/app/auth.css";
import "../dashboard/dashboard.css";
import "../operations/operations.css";
import "./spots.css";

type Filter = "all" | "candidate" | "kept" | "rejected";

// Spot Finder — Step 1: keyword detection & review (Lot A). We crawl the user's
// own site (homepage + pages one click away), extract the <h1> headings as
// keyword candidates, and let them review/edit/prune the list. Only "kept"
// keywords will feed the paid SERP lookup in the next step — so this screen is
// where the user controls both relevance and cost.
export default function SpotsPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [projects, setProjects] = useState<ProjectView[]>([]);
  const [projectId, setProjectId] = useState<string | null>(null);
  const [data, setData] = useState<SpotKeywordsResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [newKw, setNewKw] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editText, setEditText] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  // Lot B — spots (SERP search over the kept keywords).
  const [spotsData, setSpotsData] = useState<SpotsResponse | null>(null);
  const [locale, setLocale] = useState("fr");
  const [spotsBusy, setSpotsBusy] = useState(false);
  const [spotsErr, setSpotsErr] = useState<string | null>(null);
  const dataforseo = !!me?.features?.dataforseo;
  // Lot F — push a spot into a link-building campaign.
  const [campaigns, setCampaigns] = useState<CampaignView[]>([]);
  const [campaignId, setCampaignId] = useState<string>("");
  const [pushMsg, setPushMsg] = useState<string | null>(null);
  const campaignsBeta = !!me?.features?.campaigns_beta;

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

  useEffect(() => {
    if (!orgId) return;
    orgApi.projects(orgId).then((res) => {
      if (res.ok) {
        setProjects(res.data);
        setProjectId((p) => p ?? res.data[0]?.id ?? null);
      }
    });
  }, [orgId]);

  const load = useCallback(async (org: string, project: string) => {
    const res = await orgApi.spotKeywords(org, project);
    if (res.ok) setData(res.data);
  }, []);

  const loadSpots = useCallback(async (org: string, project: string) => {
    const res = await orgApi.spots(org, project);
    if (res.ok) setSpotsData(res.data);
  }, []);

  useEffect(() => {
    setData(null);
    setSpotsData(null);
    setFilter("all");
    setPushMsg(null);
    setCampaignId("");
    if (orgId && projectId) {
      void load(orgId, projectId);
      void loadSpots(orgId, projectId);
    }
  }, [orgId, projectId, load, loadSpots]);

  useEffect(() => {
    if (!orgId || !projectId || !campaignsBeta) return;
    orgApi.campaigns(orgId).then((res) => {
      if (res.ok) setCampaigns(res.data.filter((c) => c.project_id === projectId));
    });
  }, [orgId, projectId, campaignsBeta]);

  // Poll while a spot search or enrichment is running.
  const spotRun = spotsData?.run ?? null;
  const enrichRun = spotsData?.enrich_run ?? null;
  const matchRun = spotsData?.match_run ?? null;
  const searching = spotRun?.status === "queued" || spotRun?.status === "running";
  const enriching = enrichRun?.status === "queued" || enrichRun?.status === "running";
  const matching = matchRun?.status === "queued" || matchRun?.status === "running";
  const busyRun = searching || enriching || matching;
  const linkavista = !!me?.features?.linkavista;
  useEffect(() => {
    if (!orgId || !projectId || !busyRun) return;
    const t = setTimeout(() => void loadSpots(orgId, projectId), 3000);
    return () => clearTimeout(t);
  }, [orgId, projectId, busyRun, spotsData, loadSpots]);

  // Poll while a detection scan is still queued/running.
  const scan = data?.scan ?? null;
  const scanning = scan?.status === "queued" || scan?.status === "running";
  useEffect(() => {
    if (!orgId || !projectId || !scanning) return;
    const t = setTimeout(() => void load(orgId, projectId), 2500);
    return () => clearTimeout(t);
  }, [orgId, projectId, scanning, data, load]);

  async function detect() {
    if (!orgId || !projectId) return;
    setErr(null);
    setBusy(true);
    const res = await orgApi.detectSpotKeywords(orgId, projectId);
    setBusy(false);
    if (!res.ok) {
      setErr(res.detail || "Could not start detection.");
      return;
    }
    setData((d) => ({ scan: res.data, keywords: d?.keywords ?? [] }));
  }

  async function setStatus(id: string, status: string) {
    if (!orgId || !projectId) return;
    const res = await orgApi.updateSpotKeyword(orgId, projectId, id, { status });
    if (res.ok) await load(orgId, projectId);
  }
  async function bulk(status: string, ids: string[]) {
    if (!orgId || !projectId || ids.length === 0) return;
    const res = await orgApi.bulkSpotKeywordStatus(orgId, projectId, ids, status);
    if (res.ok) await load(orgId, projectId);
  }
  async function remove(id: string) {
    if (!orgId || !projectId) return;
    const res = await orgApi.deleteSpotKeyword(orgId, projectId, id);
    if (res.ok) await load(orgId, projectId);
  }
  async function addManual(e: FormEvent) {
    e.preventDefault();
    if (!orgId || !projectId) return;
    const kw = newKw.trim();
    if (!kw) return;
    setErr(null);
    const res = await orgApi.addSpotKeyword(orgId, projectId, kw);
    if (!res.ok) {
      setErr(res.detail || "Could not add keyword.");
      return;
    }
    setNewKw("");
    await load(orgId, projectId);
  }
  function startEdit(k: SpotKeywordView) {
    setEditingId(k.id);
    setEditText(k.keyword);
  }
  async function saveEdit(id: string) {
    if (!orgId || !projectId) return;
    const kw = editText.trim();
    if (!kw) return;
    const res = await orgApi.updateSpotKeyword(orgId, projectId, id, { keyword: kw });
    if (!res.ok) {
      setErr(res.detail || "Could not rename keyword.");
      return;
    }
    setEditingId(null);
    setEditText("");
    await load(orgId, projectId);
  }

  async function search() {
    if (!orgId || !projectId) return;
    setSpotsErr(null);
    setSpotsBusy(true);
    const res = await orgApi.searchSpots(orgId, projectId, locale);
    setSpotsBusy(false);
    if (!res.ok) {
      setSpotsErr(res.detail || "Could not start the search.");
      return;
    }
    setSpotsData((d) => ({
      run: res.data,
      enrich_run: d?.enrich_run ?? null,
      match_run: d?.match_run ?? null,
      spots: d?.spots ?? [],
      forums: d?.forums ?? [],
    }));
  }

  async function enrich() {
    if (!orgId || !projectId) return;
    setSpotsErr(null);
    setSpotsBusy(true);
    const res = await orgApi.enrichSpots(orgId, projectId);
    setSpotsBusy(false);
    if (!res.ok) {
      setSpotsErr(res.detail || "Could not start enrichment.");
      return;
    }
    setSpotsData((d) => ({
      run: d?.run ?? null,
      enrich_run: res.data,
      match_run: d?.match_run ?? null,
      spots: d?.spots ?? [],
      forums: d?.forums ?? [],
    }));
  }

  async function match() {
    if (!orgId || !projectId) return;
    setSpotsErr(null);
    setSpotsBusy(true);
    const res = await orgApi.matchSpots(orgId, projectId);
    setSpotsBusy(false);
    if (!res.ok) {
      setSpotsErr(res.detail || "Could not start the marketplace match.");
      return;
    }
    setSpotsData((d) => ({
      run: d?.run ?? null,
      enrich_run: d?.enrich_run ?? null,
      match_run: res.data,
      spots: d?.spots ?? [],
      forums: d?.forums ?? [],
    }));
  }

  async function pushToCampaign(spotId: string, domain: string) {
    if (!orgId || !projectId || !campaignId) return;
    setPushMsg(null);
    const res = await orgApi.spotToCampaign(orgId, projectId, spotId, campaignId);
    if (!res.ok) {
      setPushMsg(res.detail || `Could not add ${domain}.`);
      return;
    }
    const camp = campaigns.find((c) => c.id === campaignId);
    setPushMsg(`Added ${domain} to ${camp?.name ?? "the campaign"} as a prospect.`);
  }

  function formatPrice(cents: number | null, currency: string | null): string {
    if (cents == null) return "–";
    const amount = (cents / 100).toLocaleString(undefined, {
      minimumFractionDigits: 0,
      maximumFractionDigits: 2,
    });
    return `${amount} ${currency ?? "EUR"}`;
  }

  function exportSpotsCsv() {
    const rows = spotsData?.spots ?? [];
    if (rows.length === 0) return;
    const header = [
      "domain",
      "quality_score",
      "keywords",
      "best_position",
      "avg_position",
      "domain_rank",
      "referring_domains",
      "spam_score",
      "organic_keywords",
      "organic_traffic",
      "marketplace",
      "buy_price_cents",
      "buy_currency",
      "example_keyword",
      "example_url",
    ];
    const body = rows.map((s) =>
      [
        s.domain,
        s.quality_score != null ? s.quality_score.toFixed(1) : "",
        s.keyword_count,
        s.best_position ?? "",
        s.avg_position != null ? s.avg_position.toFixed(1) : "",
        s.domain_rank ?? "",
        s.referring_domains ?? "",
        s.spam_score ?? "",
        s.organic_keywords ?? "",
        s.organic_traffic ?? "",
        s.marketplace ?? "",
        s.buy_price_cents ?? "",
        s.buy_currency ?? "",
        s.example_keyword ?? "",
        s.example_url ?? "",
      ]
        .map((v) => `"${String(v).replace(/"/g, '""')}"`)
        .join(","),
    );
    const csv = [header.join(","), ...body].join("\n");
    const url = URL.createObjectURL(new Blob([csv], { type: "text/csv" }));
    const a = document.createElement("a");
    a.href = url;
    a.download = "spots.csv";
    a.click();
    URL.revokeObjectURL(url);
  }

  const keywords = data?.keywords ?? [];
  const counts = useMemo(
    () => ({
      all: keywords.length,
      candidate: keywords.filter((k) => k.status === "candidate").length,
      kept: keywords.filter((k) => k.status === "kept").length,
      rejected: keywords.filter((k) => k.status === "rejected").length,
    }),
    [keywords],
  );
  const shown = keywords.filter((k) => filter === "all" || k.status === filter);
  const candidateIds = keywords.filter((k) => k.status === "candidate").map((k) => k.id);

  function scanStatusLine(): string {
    if (!scan) return "No detection run yet. Detect keywords to scan your site’s headings.";
    switch (scan.status) {
      case "queued":
        return "Queued — your scan will start shortly…";
      case "running":
        return `Scanning your site… ${scan.pages_scanned} page${scan.pages_scanned === 1 ? "" : "s"} so far.`;
      case "error":
        return scan.error ? `Detection failed: ${scan.error}` : "Detection failed.";
      case "done":
        return `Scanned ${scan.pages_scanned} page${scan.pages_scanned === 1 ? "" : "s"}, found ${scan.h1_count} heading${scan.h1_count === 1 ? "" : "s"} → ${scan.keywords_found} new keyword${scan.keywords_found === 1 ? "" : "s"}.`;
      default:
        return "";
    }
  }

  if (loading) return <PageLoading />;
  if (!me) return null;

  const FILTERS: { key: Filter; label: string }[] = [
    { key: "all", label: `All (${counts.all})` },
    { key: "candidate", label: `To review (${counts.candidate})` },
    { key: "kept", label: `Kept (${counts.kept})` },
    { key: "rejected", label: `Rejected (${counts.rejected})` },
  ];

  return (
    <main className="dash">
      <header className="page-head">
        <h1>
          Spot Finder <span className="sf-beta">BETA</span>
        </h1>
        <p className="muted">
          Step 1 — Keywords. We read your site’s own headings to suggest the topics you rank around.
          Review and keep the ones worth prospecting; only kept keywords feed the next step.
        </p>
      </header>

      {me.organizations.length === 0 ? (
        <p className="muted">Create a workspace first.</p>
      ) : (
        <>
          <section className="panel">
            <div className="panel-head">
              <h2>Detect from your site</h2>
              {projects.length > 0 && (
                <select
                  aria-label="Project"
                  value={projectId ?? ""}
                  onChange={(e) => setProjectId(e.target.value)}
                >
                  {projects.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}, {p.domain}
                    </option>
                  ))}
                </select>
              )}
            </div>

            {projects.length === 0 ? (
              <p className="muted">Create a project first — Spot Finder scans that project’s domain.</p>
            ) : (
              <>
                <p className="muted" style={{ marginBottom: 12 }}>
                  We crawl your homepage and the pages one click away, then turn their{" "}
                  <code>&lt;h1&gt;</code> headings into keyword candidates. This scans your own site
                  only — no external cost.
                </p>
                <div className="sf-detect-row">
                  <button className="btn primary" onClick={() => void detect()} disabled={busy || scanning}>
                    {scanning ? "Scanning…" : busy ? "Starting…" : "Detect keywords"}
                  </button>
                  <span className={`sf-status${scan?.status === "error" ? " sf-status-error" : ""}`} role="status">
                    {scanning && <span className="sf-spinner" aria-hidden="true" />}
                    {scanStatusLine()}
                  </span>
                </div>
                {err && (
                  <div className="auth-error" role="alert" style={{ marginTop: 12 }}>
                    {err}
                  </div>
                )}
              </>
            )}
          </section>

          {projects.length > 0 && (
            <section className="panel" style={{ marginTop: 16 }}>
              <div className="panel-head">
                <h2>Your keywords</h2>
                <div className="sf-tabs" role="tablist" aria-label="Filter keywords">
                  {FILTERS.map((f) => (
                    <button
                      key={f.key}
                      role="tab"
                      aria-selected={filter === f.key}
                      className={`sf-tab${filter === f.key ? " active" : ""}`}
                      onClick={() => setFilter(f.key)}
                    >
                      {f.label}
                    </button>
                  ))}
                </div>
              </div>

              <div className="sf-toolbar">
                <form onSubmit={addManual} className="sf-add">
                  <input
                    aria-label="Add a keyword"
                    value={newKw}
                    onChange={(e) => setNewKw(e.target.value)}
                    placeholder="Add a keyword by hand…"
                  />
                  <button className="btn sm" type="submit" disabled={!newKw.trim()}>
                    Add
                  </button>
                </form>
                {candidateIds.length > 0 && (
                  <div className="sf-bulk">
                    <button className="btn sm" onClick={() => void bulk("kept", candidateIds)}>
                      Keep all to review
                    </button>
                    <button className="btn sm" onClick={() => void bulk("rejected", candidateIds)}>
                      Reject all to review
                    </button>
                  </div>
                )}
              </div>

              {keywords.length === 0 ? (
                <p className="muted">
                  No keywords yet. Run detection above, or add one by hand.
                </p>
              ) : shown.length === 0 ? (
                <p className="muted">Nothing in this view.</p>
              ) : (
                <ul className="sf-list">
                  {shown.map((k) => (
                    <li key={k.id} className={`sf-item sf-${k.status}`}>
                      <div className="sf-item-main">
                        {editingId === k.id ? (
                          <input
                            className="sf-edit"
                            aria-label="Edit keyword"
                            value={editText}
                            onChange={(e) => setEditText(e.target.value)}
                            onKeyDown={(e) => {
                              if (e.key === "Enter") void saveEdit(k.id);
                              if (e.key === "Escape") setEditingId(null);
                            }}
                            autoFocus
                          />
                        ) : (
                          <span className="sf-kw">{k.keyword}</span>
                        )}
                        <span className="sf-meta">
                          <span className={`sf-pill sf-pill-${k.status}`}>{k.status}</span>
                          {k.source === "manual" && <span className="sf-tag">added</span>}
                          {k.origin_url && (
                            <a
                              className="sf-origin"
                              href={k.origin_url}
                              target="_blank"
                              rel="noopener noreferrer"
                              title={k.origin_url}
                            >
                              source ↗
                            </a>
                          )}
                        </span>
                      </div>
                      <div className="sf-actions">
                        {editingId === k.id ? (
                          <>
                            <button className="btn sm" onClick={() => void saveEdit(k.id)}>
                              Save
                            </button>
                            <button className="btn sm ghost" onClick={() => setEditingId(null)}>
                              Cancel
                            </button>
                          </>
                        ) : (
                          <>
                            <button
                              className={`btn sm${k.status === "kept" ? " primary" : ""}`}
                              aria-pressed={k.status === "kept"}
                              onClick={() => void setStatus(k.id, k.status === "kept" ? "candidate" : "kept")}
                            >
                              Keep
                            </button>
                            <button
                              className="btn sm ghost"
                              aria-pressed={k.status === "rejected"}
                              onClick={() =>
                                void setStatus(k.id, k.status === "rejected" ? "candidate" : "rejected")
                              }
                            >
                              Reject
                            </button>
                            <button className="btn sm ghost" onClick={() => startEdit(k)}>
                              Edit
                            </button>
                            <button
                              className="btn sm ghost"
                              aria-label={`Delete ${k.keyword}`}
                              onClick={() => void remove(k.id)}
                            >
                              ✕
                            </button>
                          </>
                        )}
                      </div>
                    </li>
                  ))}
                </ul>
              )}

              {counts.kept > 0 && (
                <p className="sf-footnote muted">
                  {counts.kept} kept keyword{counts.kept === 1 ? "" : "s"} feed the spot search below.
                </p>
              )}
            </section>
          )}

          {projects.length > 0 && (
            <section className="panel sf-spots" style={{ marginTop: 16 }}>
              <div className="panel-head">
                <h2>Spots — who ranks for your keywords</h2>
                <div className="sf-detect-row">
                  <select
                    aria-label="Search locale"
                    value={locale}
                    onChange={(e) => setLocale(e.target.value)}
                    disabled={searching}
                  >
                    <option value="fr">France · French</option>
                    <option value="en">United States · English</option>
                  </select>
                  <button
                    className="btn primary"
                    onClick={() => void search()}
                    disabled={spotsBusy || busyRun || counts.kept === 0 || !dataforseo}
                  >
                    {searching ? "Searching…" : "Find spots"}
                  </button>
                  <button
                    className="btn sm"
                    onClick={() => void enrich()}
                    disabled={spotsBusy || busyRun || (spotsData?.spots.length ?? 0) === 0 || !dataforseo}
                    title="Fetch authority, traffic and backlink metrics for the top spots and score them"
                  >
                    {enriching ? "Scoring…" : "Enrich top spots"}
                  </button>
                  <button
                    className="btn sm"
                    onClick={() => void match()}
                    disabled={spotsBusy || busyRun || (spotsData?.spots.length ?? 0) === 0 || !linkavista}
                    title="Check which spots can be bought on your link marketplaces"
                  >
                    {matching ? "Matching…" : "Match marketplaces"}
                  </button>
                </div>
              </div>

              <p className="muted" style={{ marginBottom: 10 }}>
                For each kept keyword we look up who ranks on Google, then surface the domains that
                rank for your keywords — relevance-first link prospects. Uses DataForSEO (paid per
                query); results are cached, so re-running is cheap.
              </p>

              {!dataforseo && (
                <div className="auth-error" role="alert" style={{ marginBottom: 10 }}>
                  DataForSEO is not configured on this deployment, so the search is disabled.
                </div>
              )}
              {dataforseo && counts.kept === 0 && (
                <p className="muted">Keep at least one keyword above to enable the search.</p>
              )}

              <p
                className={`sf-status${spotRun?.status === "error" ? " sf-status-error" : ""}`}
                role="status"
              >
                {searching && <span className="sf-spinner" aria-hidden="true" />}
                {spotRun
                  ? spotRun.status === "error"
                    ? `Search failed: ${spotRun.error ?? "unknown error"}`
                    : searching
                      ? "Searching the SERPs…"
                      : `Searched ${spotRun.keywords_queried} keyword${spotRun.keywords_queried === 1 ? "" : "s"} (${spotRun.serp_queries} billed) → ${spotRun.spots_found} spot${spotRun.spots_found === 1 ? "" : "s"}.`
                  : "No search yet."}
              </p>
              {enrichRun && (
                <p
                  className={`sf-status${enrichRun.status === "error" ? " sf-status-error" : ""}`}
                  role="status"
                >
                  {enriching && <span className="sf-spinner" aria-hidden="true" />}
                  {enrichRun.status === "error"
                    ? `Enrichment failed: ${enrichRun.error ?? "unknown error"}`
                    : enriching
                      ? "Scoring the top spots…"
                      : `Scored ${enrichRun.spots_enriched} spot${enrichRun.spots_enriched === 1 ? "" : "s"} (${enrichRun.provider_queries} queries billed).`}
                </p>
              )}
              {matchRun && (
                <p
                  className={`sf-status${matchRun.status === "error" ? " sf-status-error" : ""}`}
                  role="status"
                >
                  {matching && <span className="sf-spinner" aria-hidden="true" />}
                  {matchRun.status === "error"
                    ? `Marketplace match failed: ${matchRun.error ?? "unknown error"}`
                    : matching
                      ? "Checking marketplaces…"
                      : `Checked ${matchRun.spots_checked} spot${matchRun.spots_checked === 1 ? "" : "s"} → ${matchRun.matches_found} buyable.`}
                </p>
              )}
              {spotsErr && (
                <div className="auth-error" role="alert" style={{ marginTop: 8 }}>
                  {spotsErr}
                </div>
              )}

              {(spotsData?.spots.length ?? 0) > 0 && (
                <>
                  <div className="sf-spots-head">
                    <span className="muted">{spotsData!.spots.length} spots</span>
                    <div className="sf-spots-head-actions">
                      {campaignsBeta && campaigns.length > 0 && (
                        <select
                          aria-label="Target campaign for prospects"
                          value={campaignId}
                          onChange={(e) => setCampaignId(e.target.value)}
                        >
                          <option value="">Add to campaign…</option>
                          {campaigns.map((c) => (
                            <option key={c.id} value={c.id}>
                              {c.name}
                            </option>
                          ))}
                        </select>
                      )}
                      <button className="btn sm ghost" onClick={exportSpotsCsv}>
                        Export CSV
                      </button>
                    </div>
                  </div>
                  {pushMsg && (
                    <p className="sf-footnote muted" role="status">
                      {pushMsg}
                    </p>
                  )}
                  <div className="bl-scroll">
                  <table className="bl-table sf-spots-table">
                    <thead>
                      <tr>
                        <th>Score</th>
                        <th>Domain</th>
                        <th>Keywords</th>
                        <th>Best</th>
                        <th title="Domain authority (DataForSEO rank 0–1000)">DR</th>
                        <th title="Estimated monthly organic traffic">Traffic</th>
                        <th title="Backlink spam score (lower is better)">Spam</th>
                        <th>Buy</th>
                        <th>Example</th>
                        {campaignsBeta && <th></th>}
                      </tr>
                    </thead>
                    <tbody>
                      {spotsData!.spots.map((s) => (
                        <tr key={s.id}>
                          <td>
                            {s.quality_score != null ? (
                              <span className="sf-score">{Math.round(s.quality_score)}</span>
                            ) : (
                              <span className="muted">–</span>
                            )}
                          </td>
                          <td className="mono">{s.domain}</td>
                          <td>
                            <b>{s.keyword_count}</b>
                          </td>
                          <td>{s.best_position ?? "-"}</td>
                          <td>{s.domain_rank ?? "–"}</td>
                          <td>{s.organic_traffic != null ? s.organic_traffic.toLocaleString() : "–"}</td>
                          <td>{s.spam_score ?? "–"}</td>
                          <td>
                            {s.buyable ? (
                              s.marketplace_url ? (
                                <a
                                  className="sf-buy"
                                  href={s.marketplace_url}
                                  target="_blank"
                                  rel="noopener noreferrer"
                                  title={`Buy on ${s.marketplace}`}
                                >
                                  {formatPrice(s.buy_price_cents, s.buy_currency)} ↗
                                </a>
                              ) : (
                                <span className="sf-buy">
                                  {formatPrice(s.buy_price_cents, s.buy_currency)}
                                </span>
                              )
                            ) : (
                              <span className="muted">–</span>
                            )}
                          </td>
                          <td className="sf-example">
                            {s.example_url ? (
                              <a href={s.example_url} target="_blank" rel="noopener noreferrer" title={s.example_url}>
                                {s.example_keyword ?? "page"} ↗
                              </a>
                            ) : (
                              (s.example_keyword ?? "-")
                            )}
                          </td>
                          {campaignsBeta && (
                            <td>
                              <button
                                className="btn sm ghost"
                                disabled={!campaignId}
                                title={campaignId ? "Add to the selected campaign" : "Pick a campaign above first"}
                                onClick={() => void pushToCampaign(s.id, s.domain)}
                              >
                                → Campaign
                              </button>
                            </td>
                          )}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                  </div>
                  {spotsData!.spots.some((s) => !s.enriched) && (
                    <p className="sf-footnote muted">
                      Scores, DR, traffic and spam appear after “Enrich top spots”. Blank cells aren’t
                      enriched yet.
                    </p>
                  )}
                </>
              )}
            </section>
          )}

          {(spotsData?.forums.length ?? 0) > 0 && (
            <section className="panel" style={{ marginTop: 16 }}>
              <div className="panel-head">
                <h2>Communities &amp; forums</h2>
                <span className="muted">{spotsData!.forums.length} found</span>
              </div>
              <p className="muted" style={{ marginBottom: 10 }}>
                Forums and communities that rank for your keywords — places your audience already
                discusses the topic. Join and contribute genuinely; links should come from real
                participation, not drops (most forums forbid that, and it is risky).
              </p>
              <div className="bl-scroll">
              <table className="bl-table sf-spots-table">
                <thead>
                  <tr>
                    <th>Community</th>
                    <th>Type</th>
                    <th>Best</th>
                    <th>Keyword</th>
                  </tr>
                </thead>
                <tbody>
                  {spotsData!.forums.map((f) => (
                    <tr key={f.id}>
                      <td className="sf-example">
                        <a href={f.url} target="_blank" rel="noopener noreferrer" title={f.url}>
                          {f.domain} ↗
                        </a>
                      </td>
                      <td>
                        <span className="sf-tag">{f.forum_kind}</span>
                      </td>
                      <td>{f.best_position ?? "–"}</td>
                      <td>{f.example_keyword ?? "–"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
              </div>
            </section>
          )}
        </>
      )}
    </main>
  );
}
