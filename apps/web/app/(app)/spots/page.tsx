"use client";

import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import {
  api,
  orgApi,
  type MeResponse,
  type ProjectView,
  type SpotKeywordsResponse,
  type SpotKeywordView,
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

  useEffect(() => {
    setData(null);
    setFilter("all");
    if (orgId && projectId) void load(orgId, projectId);
  }, [orgId, projectId, load]);

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

  if (loading)
    return (
      <div className="auth-wrap">
        <p style={{ color: "var(--muted)" }}>Loading…</p>
      </div>
    );
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
                  {counts.kept} kept keyword{counts.kept === 1 ? "" : "s"} will feed the SERP lookup in
                  the next step (coming soon).
                </p>
              )}
            </section>
          )}
        </>
      )}
    </main>
  );
}
