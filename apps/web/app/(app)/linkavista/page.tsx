"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import {
  api,
  orgApi,
  type LinkavistaSite,
  type LinkavistaSitesPage,
  type MeResponse,
} from "@/lib/api";
import { selectedOrg, selectedOrgId } from "@/lib/workspace";
import "@/app/auth.css";
import "../dashboard/dashboard.css";
import "./linkavista.css";

// LinkAvista marketplace catalogue (Lot 2): a read-only browse of buyable
// placements. Gated on `features.linkavista` (API key configured server-side).
// Ordering (Lot 3) is not here yet — this only surfaces inventory + metrics.
const fmtCompact = (n: number | null | undefined) =>
  n == null ? "—" : Intl.NumberFormat("en", { notation: "compact", maximumFractionDigits: 1 }).format(n);
const euros = (cents: number | null | undefined) =>
  cents == null ? "—" : `€${Math.round(cents / 100).toLocaleString("en")}`;

export default function LinkavistaPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

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

  useEffect(() => {
    api.me().then((res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      // Env-gated: no API key configured -> nothing here, back to dashboard.
      if (!res.data.features?.linkavista) {
        router.replace("/dashboard");
        return;
      }
      setMe(res.data);
      setOrgId(selectedOrgId(res.data.organizations));
      setLoading(false);
    });
  }, [router]);

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

  if (loading)
    return (
      <div className="auth-wrap">
        <p style={{ color: "var(--muted)" }}>Loading…</p>
      </div>
    );
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
              value={q}
              onChange={(e) => setQ(e.target.value)}
            />
            <input
              className="lav-num"
              type="number"
              min="0"
              placeholder="DA min"
              value={daMin}
              onChange={(e) => setDaMin(e.target.value)}
            />
            <input
              className="lav-num"
              type="number"
              min="0"
              placeholder="TF min"
              value={tfMin}
              onChange={(e) => setTfMin(e.target.value)}
            />
            <input
              className="lav-num"
              type="number"
              min="0"
              placeholder="Max €"
              value={priceMax}
              onChange={(e) => setPriceMax(e.target.value)}
            />
            <select value={language} onChange={(e) => setLanguage(e.target.value)}>
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
          Prices are in EUR, excl. VAT, as listed by LinkAvista. Placing an order from here is coming next;
          for now this surfaces live inventory and metrics.
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
                <SiteCard key={s.id} site={s} />
              ))}
            </div>
            <div className="pager" style={{ marginTop: 16 }}>
              <button className="btn sm" disabled={page <= 1} onClick={() => setPage((p) => Math.max(1, p - 1))}>
                Prev
              </button>
              <span className="muted" style={{ margin: "0 10px" }}>
                Page {result?.current_page ?? page} / {lastPage}
              </span>
              <button
                className="btn sm"
                disabled={page >= lastPage}
                onClick={() => setPage((p) => p + 1)}
              >
                Next
              </button>
            </div>
          </>
        )}
      </section>
    </main>
  );
}

function SiteCard({ site }: { site: LinkavistaSite }) {
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
          DA <b>{m?.da ?? "—"}</b>
        </span>
        <span className="lav-m">
          TF <b>{m?.tf ?? "—"}</b>
        </span>
        <span className="lav-m">
          CF <b>{m?.cf ?? "—"}</b>
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
        {modes && (
          <div className="lav-modes">
            {modes.platform && <span className="lav-mode">writing</span>}
            {modes.advertiser && <span className="lav-mode">self</span>}
          </div>
        )}
      </div>
    </div>
  );
}
