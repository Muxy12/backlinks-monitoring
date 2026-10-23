"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import {
  api,
  orgApi,
  type GscSite,
  type GscStatus,
  type MeResponse,
  type ProjectView,
} from "@/lib/api";
import { selectedOrgId } from "@/lib/workspace";
import "@/app/auth.css";
import "../../dashboard/dashboard.css";
import "../../operations/operations.css";

// Settings → Integrations. BETA: Google Search Console, connected per project
// (a project = one domain = one GSC property). Powers Post-Link Impact.
export default function IntegrationsPage() {
  const router = useRouter();
  const params = useSearchParams();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [projects, setProjects] = useState<ProjectView[]>([]);
  const [projectId, setProjectId] = useState<string>("");
  const [gsc, setGsc] = useState<GscStatus | null>(null);
  const [sites, setSites] = useState<GscSite[] | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api.me().then((res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      setMe(res.data);
      setOrgId(selectedOrgId(res.data.organizations));
    });
  }, [router]);

  // Reflect the OAuth callback redirect (?gsc=connected|denied|error|unavailable).
  useEffect(() => {
    const g = params.get("gsc");
    if (!g) return;
    setMsg(
      g === "connected"
        ? "Google Search Console connected. Pick the property below."
        : g === "denied"
          ? "Connection cancelled."
          : g === "unavailable"
            ? "Search Console integration is not configured on this deployment."
            : "Something went wrong connecting Search Console.",
    );
  }, [params]);

  useEffect(() => {
    if (!orgId) return;
    orgApi.projects(orgId).then((r) => {
      if (r.ok) {
        setProjects(r.data);
        setProjectId((p) => p || r.data[0]?.id || "");
      }
    });
  }, [orgId]);

  const loadStatus = useCallback(async () => {
    if (!orgId || !projectId) return;
    const r = await orgApi.gscStatus(orgId, projectId);
    if (r.ok) setGsc(r.data);
    setSites(null);
  }, [orgId, projectId]);

  useEffect(() => {
    void loadStatus();
  }, [loadStatus]);

  async function connect() {
    if (!orgId || !projectId) return;
    setBusy(true);
    const r = await orgApi.gscAuthUrl(orgId, projectId);
    setBusy(false);
    if (r.ok) window.location.href = r.data.url;
    else setMsg(r.detail);
  }

  async function loadSites() {
    if (!orgId || !projectId) return;
    setBusy(true);
    const r = await orgApi.gscSites(orgId, projectId);
    setBusy(false);
    if (r.ok) setSites(r.data);
    else setMsg(r.detail);
  }

  async function pickSite(siteUrl: string) {
    if (!orgId || !projectId) return;
    const r = await orgApi.gscSetSite(orgId, projectId, siteUrl);
    if (r.ok) {
      setMsg("Property mapped.");
      await loadStatus();
    } else setMsg(r.detail);
  }

  async function disconnect() {
    if (!orgId || !projectId) return;
    if (!confirm("Disconnect Google Search Console from this project?")) return;
    const r = await orgApi.gscDisconnect(orgId, projectId);
    if (r.ok) {
      setMsg("Disconnected.");
      await loadStatus();
    } else setMsg(r.detail);
  }

  if (!me) return null;

  return (
    <main className="dash">
      <header className="page-head">
        <h1>Integrations</h1>
        <p className="muted">Connect external data sources that power LinkPulz insights.</p>
      </header>

      {!orgId ? (
        <p className="muted">Create a workspace first.</p>
      ) : !me.features?.gsc ? (
        <section className="panel" style={{ maxWidth: 680 }}>
          <div className="panel-head">
            <h2>
              Google Search Console{" "}
              <span
                style={{
                  fontSize: 11,
                  fontWeight: 700,
                  padding: "2px 7px",
                  borderRadius: 999,
                  background: "var(--surface-3)",
                  color: "var(--muted)",
                  verticalAlign: "middle",
                }}
              >
                COMING SOON
              </span>
            </h2>
          </div>
          <p className="muted">
            Post-Link Impact, measuring the visibility, ranking, crawl and indexation response
            after each backlink is acquired, is coming soon. You'll be able to connect your own
            Google Search Console property here shortly.
          </p>
        </section>
      ) : (
        <section className="panel" style={{ maxWidth: 680 }}>
          <div className="panel-head">
            <h2>
              Google Search Console{" "}
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
            </h2>
          </div>
          <p className="muted">
            Connect a project to its Search Console property to measure visibility and ranking
            response after backlink acquisitions (Post-Link Impact).
          </p>

          {msg && <p className="muted">{msg}</p>}

          <label className="auth-field" style={{ maxWidth: 360 }}>
            <span>Project</span>
            <select value={projectId} onChange={(e) => setProjectId(e.target.value)}>
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}, {p.domain}
                </option>
              ))}
            </select>
          </label>

          {!gsc ? (
            <p className="muted">Loading…</p>
          ) : !gsc.configured ? (
            <p className="muted">
              Search Console isn’t configured on this deployment yet. Set the Google OAuth
              credentials to enable it.
            </p>
          ) : !gsc.connected ? (
            <button className="btn primary" onClick={() => void connect()} disabled={busy}>
              {busy ? "…" : "Connect Google Search Console"}
            </button>
          ) : (
            <div>
              <div className="detail-grid" style={{ margin: "8px 0" }}>
                <span>Status</span>
                <span style={{ color: "var(--live)" }}>Connected</span>
                <span>Property</span>
                <span className="mono">{gsc.site_url ?? "Not mapped"}</span>
                {gsc.google_email && (
                  <>
                    <span>Account</span>
                    <span>{gsc.google_email}</span>
                  </>
                )}
              </div>

              {!gsc.site_url && (
                <div style={{ margin: "8px 0" }}>
                  {sites == null ? (
                    <button className="btn sm" onClick={() => void loadSites()} disabled={busy}>
                      {busy ? "Loading…" : "Choose a property"}
                    </button>
                  ) : sites.length === 0 ? (
                    <p className="muted">No properties found for this Google account.</p>
                  ) : (
                    <ul className="ops-list">
                      {sites.map((s) => (
                        <li key={s.site_url}>
                          <div className="ops-row-wrap">
                            <span className="ops-row" style={{ cursor: "default" }}>
                              <span className="mono">{s.site_url}</span>
                              {s.permission_level && (
                                <span className="muted">{s.permission_level}</span>
                              )}
                            </span>
                            <div className="ops-actions">
                              <button className="btn sm primary" onClick={() => void pickSite(s.site_url)}>
                                Use
                              </button>
                            </div>
                          </div>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              )}

              <button
                className="btn sm danger"
                style={{ marginTop: 8 }}
                onClick={() => void disconnect()}
              >
                Disconnect
              </button>
            </div>
          )}
        </section>
      )}
    </main>
  );
}
