"use client";
import { PageLoading } from "@/components/PageLoading";

import { useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import {
  api,
  orgApi,
  type LinkavistaOrderOutcome,
  type LinkavistaSite,
  type LinkavistaSitesPage,
  type MeResponse,
  type ProjectView,
} from "@/lib/api";
import { selectedOrg, selectedOrgId } from "@/lib/workspace";
import "@/app/auth.css";
import "../dashboard/dashboard.css";
import "./linkavista.css";

// LinkAvista marketplace catalogue (Lot 2) + sandbox order flow (Lot 3a). The
// order modal places a SANDBOX order, a dry-run preview with no real order and
// no debit. Live ordering (real spend) is a later, explicitly confirmed step.
const fmtCompact = (n: number | null | undefined) =>
  n == null ? "-" : Intl.NumberFormat("en", { notation: "compact", maximumFractionDigits: 1 }).format(n);
const euros = (cents: number | null | undefined) =>
  cents == null ? "-" : `€${Math.round(cents / 100).toLocaleString("en")}`;

export default function LinkavistaPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [projects, setProjects] = useState<ProjectView[]>([]);

  // Filters
  const [q, setQ] = useState("");
  const [daMin, setDaMin] = useState("");
  const [tfMin, setTfMin] = useState("");
  const [priceMax, setPriceMax] = useState(""); // euros, converted to cents on call
  const [language, setLanguage] = useState("");
  const [page, setPage] = useState(1);

  const [result, setResult] = useState<LinkavistaSitesPage | null>(null);
  const [fetching, setFetching] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Order modal
  const [orderSite, setOrderSite] = useState<LinkavistaSite | null>(null);

  useEffect(() => {
    api.me().then((res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      if (!res.data.features?.linkavista) {
        router.replace("/dashboard");
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
      if (res.ok) setProjects(res.data);
    });
  }, [orgId]);

  // Reset to page 1 whenever a filter changes.
  useEffect(() => {
    setPage(1);
  }, [q, daMin, tfMin, priceMax, language]);

  // Debounced catalogue fetch.
  useEffect(() => {
    if (!orgId) return;
    const t = setTimeout(() => {
      setFetching(true);
      setError(null);
      const num = (s: string) => {
        const n = parseInt(s, 10);
        return Number.isFinite(n) ? n : undefined;
      };
      const priceEuros = num(priceMax);
      orgApi
        .linkavistaSites(orgId, {
          q: q.trim() || undefined,
          daMin: num(daMin),
          tfMin: num(tfMin),
          priceMax: priceEuros != null ? priceEuros * 100 : undefined,
          language: language || undefined,
          page,
          perPage: 24,
        })
        .then((res) => {
          setFetching(false);
          if (!res.ok) {
            setResult(null);
            setError(res.detail);
            return;
          }
          setResult(res.data);
        });
    }, 250);
    return () => clearTimeout(t);
  }, [orgId, q, daMin, tfMin, priceMax, language, page]);

  if (loading) return <PageLoading />;
  if (!me) return null;

  const sites = result?.data ?? [];
  const lastPage = result?.last_page ?? 1;
  const total = result?.total ?? 0;

  return (
    <main className="dash">
      <header className="page-head">
        <h1>
          Marketplace{" "}
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
          Browse the LinkAvista catalogue of buyable placements · {selectedOrg(me.organizations)?.name ?? "No workspace"}
        </p>
      </header>

      <section className="panel">
        <div className="panel-head">
          <h2>Catalogue{total ? ` · ${total.toLocaleString("en")} sites` : ""}</h2>
          <div className="panel-actions lav-filters">
            <input
              className="bl-search"
              placeholder="Search domain"
              aria-label="Search domain"
              value={q}
              onChange={(e) => setQ(e.target.value)}
            />
            <input
              className="lav-num"
              type="number"
              min="0"
              placeholder="DA min"
              aria-label="Minimum Domain Authority"
              value={daMin}
              onChange={(e) => setDaMin(e.target.value)}
            />
            <input
              className="lav-num"
              type="number"
              min="0"
              placeholder="TF min"
              aria-label="Minimum Trust Flow"
              value={tfMin}
              onChange={(e) => setTfMin(e.target.value)}
            />
            <input
              className="lav-num"
              type="number"
              min="0"
              placeholder="Max €"
              aria-label="Maximum price in euros"
              value={priceMax}
              onChange={(e) => setPriceMax(e.target.value)}
            />
            <select
              value={language}
              aria-label="Filter by language"
              onChange={(e) => setLanguage(e.target.value)}
            >
              <option value="">Any language</option>
              <option value="en">English</option>
              <option value="fr">French</option>
              <option value="de">German</option>
              <option value="es">Spanish</option>
              <option value="it">Italian</option>
            </select>
          </div>
        </div>

        <p className="lav-note">
          Prices are in EUR, excl. VAT, as listed by LinkAvista. Ordering runs in <b>sandbox</b> for now, placing an
          order shows a price preview and does not spend anything.
        </p>

        {error ? (
          <p className="muted" style={{ marginTop: 16 }}>
            Couldn&apos;t load the catalogue: {error}
          </p>
        ) : sites.length === 0 ? (
          <p className="muted" style={{ marginTop: 16 }}>
            {fetching ? "Loading catalogue…" : "No sites match these filters."}
          </p>
        ) : (
          <>
            <div className="lav-grid" style={{ opacity: fetching ? 0.6 : 1 }}>
              {sites.map((s) => (
                <SiteCard key={s.id} site={s} onOrder={() => setOrderSite(s)} />
              ))}
            </div>
            <div className="pager" style={{ marginTop: 16 }}>
              <button className="btn sm" disabled={page <= 1} onClick={() => setPage((p) => Math.max(1, p - 1))}>
                Prev
              </button>
              <span className="muted" style={{ margin: "0 10px" }}>
                Page {result?.current_page ?? page} / {lastPage}
              </span>
              <button className="btn sm" disabled={page >= lastPage} onClick={() => setPage((p) => p + 1)}>
                Next
              </button>
            </div>
          </>
        )}
      </section>

      {orderSite && orgId && (
        <OrderModal
          org={orgId}
          site={orderSite}
          projects={projects}
          onClose={() => setOrderSite(null)}
        />
      )}
    </main>
  );
}

function SiteCard({ site, onOrder }: { site: LinkavistaSite; onOrder: () => void }) {
  const m = site.metrics;
  const modes = site.writing_modes;
  return (
    <div className="lav-card">
      <div>
        <div className="lav-dom">{site.domain}</div>
        {(site.theme || site.language) && (
          <div className="lav-theme">
            {site.theme ?? ""}
            {site.theme && site.language ? " · " : ""}
            {site.language ? site.language.toUpperCase() : ""}
          </div>
        )}
      </div>
      <div className="lav-metrics">
        <span className="lav-m">
          DA <b>{m?.da ?? "-"}</b>
        </span>
        <span className="lav-m">
          TF <b>{m?.tf ?? "-"}</b>
        </span>
        <span className="lav-m">
          CF <b>{m?.cf ?? "-"}</b>
        </span>
        <span className="lav-m">
          RD <b>{fmtCompact(m?.rd)}</b>
        </span>
        <span className="lav-m">
          Traffic <b>{fmtCompact(m?.total_traffic)}</b>
        </span>
      </div>
      <div className="lav-foot">
        <div className="lav-price">
          {euros(site.pricing?.price)} <small>excl. VAT</small>
        </div>
        <button className="btn sm primary" onClick={onOrder} disabled={!(modes?.platform ?? true)}>
          Order
        </button>
      </div>
    </div>
  );
}

function defaultPublishDate(): string {
  const d = new Date();
  d.setDate(d.getDate() + 14);
  return d.toISOString().slice(0, 10);
}

function OrderModal({
  org,
  site,
  projects,
  onClose,
}: {
  org: string;
  site: LinkavistaSite;
  projects: ProjectView[];
  onClose: () => void;
}) {
  const [projectId, setProjectId] = useState(projects[0]?.id ?? "");
  const [targetUrl, setTargetUrl] = useState("");
  const [anchor, setAnchor] = useState("");
  const [title, setTitle] = useState("");
  const [brief, setBrief] = useState("");
  const [redaction, setRedaction] = useState("classique");
  const [publishDate, setPublishDate] = useState(defaultPublishDate());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [outcome, setOutcome] = useState<LinkavistaOrderOutcome | null>(null);
  const dialogRef = useRef<HTMLDivElement>(null);

  // Accessible dialog: move focus in on open, close on Escape, restore focus.
  useEffect(() => {
    const prev = document.activeElement as HTMLElement | null;
    dialogRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
      prev?.focus?.();
    };
  }, [onClose]);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    if (!projectId) {
      setError("Pick a project.");
      return;
    }
    setBusy(true);
    const res = await orgApi.createLinkavistaOrder(org, {
      site_id: site.id,
      project_id: projectId,
      target_url: targetUrl.trim(),
      anchor_text: anchor.trim(),
      article_title: title.trim(),
      brief: brief.trim(),
      redaction_option: redaction,
      publish_date: publishDate,
    });
    setBusy(false);
    if (!res.ok) {
      setError(res.detail);
      return;
    }
    setOutcome(res.data);
  }

  return (
    <div className="lav-modal-backdrop" onClick={onClose}>
      <div
        className="lav-modal"
        ref={dialogRef}
        tabIndex={-1}
        role="dialog"
        aria-modal="true"
        aria-labelledby="lav-modal-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="lav-modal-head">
          <h2 id="lav-modal-title">Order a link on {site.domain}</h2>
          <button className="btn sm" onClick={onClose} aria-label="Close">
            ✕
          </button>
        </div>

        {outcome ? (
          <div className="lav-outcome">
            <div className={`lav-outcome-badge${outcome.sandbox ? " sandbox" : ""}`}>
              {outcome.sandbox ? "Sandbox, no order placed" : "Order placed"}
            </div>
            <p>
              {outcome.sandbox
                ? "This is a price preview. Nothing was ordered and nothing was charged."
                : outcome.message ?? "Your order was submitted."}
            </p>
            <dl className="lav-outcome-grid">
              <div>
                <dt>{outcome.sandbox ? "Would cost" : "Price"}</dt>
                <dd>
                  {euros(outcome.would_debit_cents ?? outcome.price_cents)}{" "}
                  <small>excl. VAT</small>
                </dd>
              </div>
              <div>
                <dt>Status</dt>
                <dd>{outcome.would_create_status ?? outcome.status ?? "-"}</dd>
              </div>
            </dl>
            <div className="lav-modal-actions">
              <button className="btn sm" onClick={onClose}>
                Close
              </button>
            </div>
          </div>
        ) : (
          <form onSubmit={submit} className="ops-form">
            {error && <div className="auth-error" role="alert">{error}</div>}
            <label className="auth-field">
              <span>Project (the site the link points to)</span>
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
              <span>Target URL</span>
              <input
                value={targetUrl}
                onChange={(e) => setTargetUrl(e.target.value)}
                placeholder="https://your-site.com/page"
                required
              />
            </label>
            <label className="auth-field">
              <span>Anchor text</span>
              <input value={anchor} onChange={(e) => setAnchor(e.target.value)} required />
            </label>
            <label className="auth-field">
              <span>Article title</span>
              <input value={title} onChange={(e) => setTitle(e.target.value)} required />
            </label>
            <label className="auth-field">
              <span>Brief (optional)</span>
              <input
                value={brief}
                onChange={(e) => setBrief(e.target.value)}
                placeholder="What the article should cover"
              />
            </label>
            <div className="ops-money">
              <label className="auth-field">
                <span>Writing option</span>
                <select value={redaction} onChange={(e) => setRedaction(e.target.value)}>
                  <option value="classique">Standard</option>
                  <option value="rapide">Fast</option>
                  <option value="geo">Geo</option>
                </select>
              </label>
              <label className="auth-field">
                <span>Publish date</span>
                <input type="date" value={publishDate} onChange={(e) => setPublishDate(e.target.value)} />
              </label>
            </div>
            <button className="auth-btn" type="submit" disabled={busy}>
              {busy ? "Checking…" : "Preview order (sandbox)"}
            </button>
          </form>
        )}
      </div>
    </div>
  );
}
