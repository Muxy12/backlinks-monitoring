"use client";
import { PageLoading } from "@/components/PageLoading";
import { confirmDialog } from "../confirm";

import { useCallback, useEffect, useMemo, useState, type FormEvent, type ReactNode } from "react";
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
  type SpotView,
  type SpotKeywordPosition,
  type SpotOrder,
  type ProjectOverview,
  type PurchaseCampaign,
  type PurchaseCampaignDetail,
  type SpotRunView,
  type GeoQueryView,
  type GeoCitationsResponse,
} from "@/lib/api";
import { selectedOrgId } from "@/lib/workspace";
import "@/app/auth.css";
import "../dashboard/dashboard.css";
import "../operations/operations.css";
import "./spots.css";

type Filter = "all" | "candidate" | "kept" | "rejected";

// Price/performance grade (from the API: pepite|a_plus|a|b|c|unrated).
const GRADE_ORDER = ["pepite", "a_plus", "a", "b", "c"] as const;
const GRADE_LABEL: Record<string, string> = {
  pepite: "Pépite",
  a_plus: "A+",
  a: "A",
  b: "B",
  c: "C",
  unrated: "—",
};
function GradeBadge({ grade }: { grade?: string }) {
  const g = grade ?? "unrated";
  return <span className={`sf-grade g-${g}`}>{GRADE_LABEL[g] ?? "—"}</span>;
}
// A 0–100 thematic score as a circular gauge (green = strong, red = weak),
// Référencime-style — the ring colour tracks the value, the number sits inside.
function ScoreDonut({ v, size = 40 }: { v?: number; size?: number }) {
  if (v == null) return <span className="sf-donut-empty">–</span>;
  const val = Math.max(0, Math.min(100, v));
  const stroke = 4;
  const r = (size - stroke) / 2;
  const circ = 2 * Math.PI * r;
  const hue = Math.round((val / 100) * 125); // 0 = red → 125 = green
  const color = `hsl(${hue} 62% 44%)`;
  return (
    <svg className="sf-donut" width={size} height={size} viewBox={`0 0 ${size} ${size}`} aria-hidden="true">
      <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="var(--surface-3)" strokeWidth={stroke} />
      <circle
        cx={size / 2}
        cy={size / 2}
        r={r}
        fill="none"
        stroke={color}
        strokeWidth={stroke}
        strokeLinecap="round"
        strokeDasharray={`${(circ * val) / 100} ${circ}`}
        transform={`rotate(-90 ${size / 2} ${size / 2})`}
      />
      <text x="50%" y="50%" dominantBaseline="central" textAnchor="middle" fontSize={size * 0.33} fontWeight={700} fill="var(--text)">
        {Math.round(val)}
      </text>
    </svg>
  );
}
function Stat({ label, v }: { label: string; v?: number | null }) {
  return (
    <div className="sf-metric">
      <b>{v != null ? v.toLocaleString() : "–"}</b>
      <span>{label}</span>
    </div>
  );
}
function TabBtn({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button type="button" onClick={onClick} className={`sf-tab${active ? " active" : ""}`}>
      {children}
    </button>
  );
}

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
  // URL-first front door: type a site URL → resolve/create its project → scan.
  const [analyzeInput, setAnalyzeInput] = useState("");
  const [analyzing, setAnalyzing] = useState(false);
  // Auto-pilot progress line shown while the URL→spots chain runs.
  const [pilot, setPilot] = useState<string | null>(null);
  // Netlinking table: grade filter (all on by default).
  const [gradeFilter, setGradeFilter] = useState<Set<string>>(
    () => new Set(GRADE_ORDER),
  );
  // "My analyses" cards (every analyzed site + its state).
  const [overview, setOverview] = useState<ProjectOverview[]>([]);
  // Spot detail drawer.
  const [detail, setDetail] = useState<SpotView | null>(null);
  const [detailKw, setDetailKw] = useState<SpotKeywordPosition[] | null>(null);
  const [detailOrders, setDetailOrders] = useState<SpotOrder[] | null>(null);
  const [detailTab, setDetailTab] = useState<"keywords" | "prices" | "orders">("keywords");
  // New-order form (order-tracking tab).
  const [orderForm, setOrderForm] = useState({ platform: "", target_url: "", price: "" });
  const [spotsBusy, setSpotsBusy] = useState(false);
  const [spotsErr, setSpotsErr] = useState<string | null>(null);
  const dataforseo = !!me?.features?.dataforseo;
  // Lot F — push a spot into a link-building campaign.
  const [campaigns, setCampaigns] = useState<CampaignView[]>([]);
  const [campaignId, setCampaignId] = useState<string>("");
  const [pushMsg, setPushMsg] = useState<string | null>(null);
  const campaignsBeta = !!me?.features?.campaigns_beta;
  // Phase 3: multi-select spots → selection bar (budget, savings, bulk actions).
  const [selected, setSelected] = useState<Set<string>>(() => new Set());
  const [selBusy, setSelBusy] = useState(false);
  // Purchase-campaign naming modal (replaces window.prompt, which throws in
  // embedded/webview browsers where prompt() is unsupported).
  const [campModal, setCampModal] = useState<{ ids: string[]; name: string } | null>(null);
  // Purchase campaigns (buy baskets).
  const [campList, setCampList] = useState<PurchaseCampaign[]>([]);
  const [campDetail, setCampDetail] = useState<PurchaseCampaignDetail | null>(null);
  function toggleSelected(id: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }
  // GEO pillar ("cited by AI").
  const [geoQueries, setGeoQueries] = useState<GeoQueryView[]>([]);
  const [geoCites, setGeoCites] = useState<GeoCitationsResponse | null>(null);
  const [newGeoQuery, setNewGeoQuery] = useState("");
  const [geoBusy, setGeoBusy] = useState(false);
  const [geoErr, setGeoErr] = useState<string | null>(null);

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

  // "My analyses" cards: load once per org…
  useEffect(() => {
    if (!orgId) return;
    let alive = true;
    orgApi.spotOverview(orgId).then((r) => {
      if (alive && r.ok) setOverview(r.data);
    });
    return () => {
      alive = false;
    };
  }, [orgId]);
  // …and poll while any site is still analyzing.
  const anyAnalyzing = overview.some((o) => o.analyzing);
  useEffect(() => {
    if (!orgId || !anyAnalyzing) return;
    const t = setInterval(() => {
      orgApi.spotOverview(orgId).then((r) => {
        if (r.ok) setOverview(r.data);
      });
    }, 4000);
    return () => clearInterval(t);
  }, [orgId, anyAnalyzing]);

  const load = useCallback(async (org: string, project: string) => {
    const res = await orgApi.spotKeywords(org, project);
    if (res.ok) setData(res.data);
  }, []);

  const loadSpots = useCallback(async (org: string, project: string) => {
    const res = await orgApi.spots(org, project);
    if (res.ok) setSpotsData(res.data);
  }, []);

  const loadGeo = useCallback(async (org: string, project: string) => {
    const [q, c] = await Promise.all([
      orgApi.geoQueries(org, project),
      orgApi.geoCitations(org, project),
    ]);
    if (q.ok) setGeoQueries(q.data.queries);
    if (c.ok) setGeoCites(c.data);
  }, []);

  useEffect(() => {
    setData(null);
    setSpotsData(null);
    setFilter("all");
    setPushMsg(null);
    setCampaignId("");
    setGeoQueries([]);
    setGeoCites(null);
    setGeoErr(null);
    setNewGeoQuery("");
    if (orgId && projectId) {
      void load(orgId, projectId);
      void loadSpots(orgId, projectId);
      void loadGeo(orgId, projectId);
      void orgApi.listPurchaseCampaigns(orgId, projectId).then((r) => setCampList(r.ok ? r.data : []));
    }
  }, [orgId, projectId, load, loadSpots, loadGeo]);

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
  const marketplace = !!me?.features?.marketplace;
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

  // URL-first auto-pilot: analyze a site by URL alone, then drive the whole
  // chain — read its topics → auto-keep them → discover matching spots — so the
  // user goes from a URL to scored, price-compared spots in one action. The
  // page's own polling effects keep the detected-topics + spots UI in sync as
  // each phase lands; this just orchestrates the phases.
  async function analyze(e?: FormEvent) {
    e?.preventDefault();
    if (!orgId) return;
    const url = analyzeInput.trim();
    if (!url) return;
    setErr(null);
    setSpotsErr(null);
    setAnalyzing(true);
    setPilot("Resolving your site…");
    const res = await orgApi.analyzeUrl(orgId, url);
    if (!res.ok) {
      setAnalyzing(false);
      setPilot(null);
      setErr(res.detail || "Could not analyze that URL.");
      return;
    }
    const pid = res.data.project_id;
    // Refresh the picker if a new project was created, then select it.
    if (res.data.created || !projects.some((p) => p.id === pid)) {
      const pl = await orgApi.projects(orgId);
      if (pl.ok) setProjects(pl.data);
    }
    setData({ scan: res.data.scan, keywords: [] });
    setProjectId(pid);
    setAnalyzeInput("");
    void orgApi.spotOverview(orgId).then((r) => {
      if (r.ok) setOverview(r.data);
    });

    try {
      // 1 — wait for the detection scan (reads the site's own topics).
      setPilot("Reading your site’s topics…");
      const keywords = await pollScanDone(orgId, pid);

      // 2 — auto-keep every detected topic so it feeds discovery.
      const candidateIds = keywords.filter((k) => k.status === "candidate").map((k) => k.id);
      let keptCount = keywords.filter((k) => k.status === "kept").length;
      if (candidateIds.length > 0) {
        setPilot("Keeping the detected topics…");
        await orgApi.bulkSpotKeywordStatus(orgId, pid, candidateIds, "kept");
        keptCount += candidateIds.length;
        await load(orgId, pid);
      }

      if (keptCount === 0) {
        setErr("We couldn’t read any topics from that site. Add a few keywords below, then analyze.");
        return;
      }
      // 3 — build the ranking. Prefer SERP (it populates the PTA/PTP thematic
      // scores from real Google positions), then overlay marketplace prices.
      // Fall back to marketplace-only discovery when no SERP source is set up.
      if (dataforseo) {
        setPilot("Finding the sites that rank on your topics…");
        const sres = await orgApi.searchSpots(orgId, pid, locale);
        if (!sres.ok) {
          setSpotsErr(sres.detail || "Could not start the SERP search.");
          return;
        }
        applyRun(sres);
        await pollSpotRunDone(orgId, pid);
        if (marketplace) {
          setPilot("Comparing marketplace prices…");
          const mres = await orgApi.matchSpots(orgId, pid);
          if (mres.ok) setSpotsData((d) => (d ? { ...d, match_run: mres.data } : d));
        }
        await loadSpots(orgId, pid);
      } else if (marketplace) {
        setPilot("Finding matching spots across your marketplaces…");
        const dres = await orgApi.discoverMarketplaceSpots(orgId, pid, locale);
        if (dres.ok) {
          applyRun(dres);
          await loadSpots(orgId, pid);
        } else {
          setSpotsErr(dres.detail || "Could not start marketplace discovery.");
        }
      } else {
        setErr("No SERP (DataForSEO) or marketplace source is configured, so we can’t build a ranking yet. Your topics are ready below.");
      }
    } catch {
      setErr("The analysis didn’t finish in time. You can continue from the detected topics below.");
    } finally {
      setPilot(null);
      setAnalyzing(false);
    }
  }

  // Poll the latest SERP spot run until it is done (or errors/times out). Used
  // by the auto-pilot to wait before overlaying marketplace prices.
  async function pollSpotRunDone(org: string, project: string) {
    for (let i = 0; i < 160; i++) {
      const r = await orgApi.spots(org, project);
      if (r.ok) {
        const st = r.data.run?.status;
        if (st === "done") return;
        if (st === "error") throw new Error("serp run failed");
      }
      await new Promise((resolve) => setTimeout(resolve, 2500));
    }
    throw new Error("timeout");
  }

  // Poll a project's detection scan until it is done (returns its keywords) or
  // errors/times out (throws). ~160 × 2.5s ≈ a generous few minutes.
  async function pollScanDone(org: string, project: string) {
    for (let i = 0; i < 160; i++) {
      const r = await orgApi.spotKeywords(org, project);
      if (r.ok) {
        const st = r.data.scan?.status;
        if (st === "done") return r.data.keywords;
        if (st === "error") throw new Error("detection failed");
      }
      await new Promise((resolve) => setTimeout(resolve, 2500));
    }
    throw new Error("timeout");
  }

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

  function applyRun(res: { data: SpotRunView }) {
    setSpotsData((d) => ({
      run: res.data,
      enrich_run: d?.enrich_run ?? null,
      match_run: d?.match_run ?? null,
      spots: d?.spots ?? [],
      forums: d?.forums ?? [],
    }));
  }

  async function discoverMarketplace() {
    if (!orgId || !projectId) return;
    setSpotsErr(null);
    setSpotsBusy(true);
    const res = await orgApi.discoverMarketplaceSpots(orgId, projectId, locale);
    setSpotsBusy(false);
    if (!res.ok) {
      setSpotsErr(res.detail || "Could not start marketplace discovery.");
      return;
    }
    applyRun(res);
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
    applyRun(res);
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

  async function monitorSpot(spotId: string) {
    if (!orgId || !projectId) return;
    setPushMsg(null);
    const res = await orgApi.monitorSpot(orgId, projectId, spotId);
    if (!res.ok) {
      setSpotsErr(res.detail || "Could not add the link to monitoring.");
      return;
    }
    setPushMsg(
      "Added to monitoring — set the exact published article URL in Backlinks once the link is live.",
    );
  }

  async function monitorSelected(ids: string[]) {
    if (!orgId || !projectId || ids.length === 0) return;
    setSelBusy(true);
    setPushMsg(null);
    let ok = 0;
    for (const id of ids) {
      const r = await orgApi.monitorSpot(orgId, projectId, id);
      if (r.ok) ok += 1;
    }
    setSelBusy(false);
    setSelected(new Set());
    setPushMsg(`Added ${ok} spot${ok === 1 ? "" : "s"} to monitoring.`);
  }

  async function addSelectedToCampaign(ids: string[]) {
    if (!orgId || !projectId || !campaignId || ids.length === 0) return;
    setSelBusy(true);
    setPushMsg(null);
    let ok = 0;
    for (const id of ids) {
      const r = await orgApi.spotToCampaign(orgId, projectId, id, campaignId);
      if (r.ok) ok += 1;
    }
    setSelBusy(false);
    setSelected(new Set());
    const camp = campaigns.find((c) => c.id === campaignId);
    setPushMsg(`Added ${ok} spot${ok === 1 ? "" : "s"} to ${camp?.name ?? "the campaign"}.`);
  }

  function createCampaignFromSelection(ids: string[]) {
    if (!orgId || !projectId || ids.length === 0) return;
    setSpotsErr(null);
    setCampModal({ ids, name: "New campaign" });
  }

  async function confirmCreateCampaign() {
    if (!orgId || !projectId || !campModal) return;
    const name = campModal.name.trim();
    if (!name) return;
    setSelBusy(true);
    setSpotsErr(null);
    setPushMsg(null);
    const res = await orgApi.createPurchaseCampaign(orgId, projectId, {
      name,
      spot_ids: campModal.ids,
    });
    setSelBusy(false);
    if (!res.ok) {
      // Keep the modal open so the user sees the error and can retry.
      setSpotsErr(res.detail || "Could not create the campaign.");
      return;
    }
    setCampModal(null);
    setSelected(new Set());
    setPushMsg(`Campaign “${res.data.name}” created with ${res.data.item_count} spot(s).`);
    void orgApi
      .listPurchaseCampaigns(orgId, projectId)
      .then((r) => setCampList(r.ok ? r.data : []));
    void openCampaign(res.data.id);
  }

  async function openCampaign(id: string) {
    if (!orgId || !projectId) return;
    setCampDetail(null);
    const r = await orgApi.purchaseCampaign(orgId, projectId, id);
    if (r.ok) setCampDetail(r.data);
  }

  function reloadCampList() {
    if (orgId && projectId)
      void orgApi.listPurchaseCampaigns(orgId, projectId).then((r) => setCampList(r.ok ? r.data : []));
  }
  async function setItemStatus(item: PurchaseCampaignDetail["items"][number], status: string) {
    if (!orgId || !projectId || !campDetail) return;
    const r = await orgApi.updateCampaignItem(orgId, projectId, campDetail.campaign.id, item.id, {
      status,
      target_url: item.target_url,
      anchor: item.anchor,
      price_cents: item.price_cents,
    });
    if (r.ok) {
      setCampDetail((d) =>
        d ? { ...d, items: d.items.map((x) => (x.id === item.id ? r.data : x)) } : d,
      );
      reloadCampList();
    }
  }
  async function removeCampaignItem(itemId: string) {
    if (!orgId || !projectId || !campDetail) return;
    const r = await orgApi.deleteCampaignItem(orgId, projectId, campDetail.campaign.id, itemId);
    if (r.ok) {
      void openCampaign(campDetail.campaign.id);
      reloadCampList();
    }
  }
  async function setCampaignStatus(status: string) {
    if (!orgId || !projectId || !campDetail) return;
    const r = await orgApi.updatePurchaseCampaign(orgId, projectId, campDetail.campaign.id, {
      name: campDetail.campaign.name,
      status,
      budget_cents: campDetail.campaign.budget_cents,
    });
    if (r.ok) {
      setCampDetail((d) => (d ? { ...d, campaign: r.data } : d));
      reloadCampList();
    }
  }
  async function deleteCampaign() {
    if (!orgId || !projectId || !campDetail) return;
    const ok = await confirmDialog({
      title: "Delete campaign",
      message: `Delete campaign “${campDetail.campaign.name}”? This cannot be undone.`,
      confirmLabel: "Delete",
      danger: true,
    });
    if (!ok) return;
    const r = await orgApi.deletePurchaseCampaign(orgId, projectId, campDetail.campaign.id);
    if (r.ok) {
      setCampDetail(null);
      reloadCampList();
    }
  }

  function openDetail(s: SpotView) {
    setDetail(s);
    setDetailTab("keywords");
    setDetailKw(null);
    setDetailOrders(null);
    setOrderForm({ platform: "", target_url: "", price: "" });
    if (orgId && projectId) {
      void orgApi.spotKeywordPositions(orgId, projectId, s.id).then((res) => {
        setDetailKw(res.ok ? res.data : []);
      });
      void orgApi.spotOrders(orgId, projectId, s.id).then((res) => {
        setDetailOrders(res.ok ? res.data : []);
      });
    }
  }

  async function addOrder() {
    if (!orgId || !projectId || !detail) return;
    const price = orderForm.price.trim();
    const res = await orgApi.createSpotOrder(orgId, projectId, detail.id, {
      status: "todo",
      platform: orderForm.platform.trim() || null,
      target_url: orderForm.target_url.trim() || null,
      price_cents: price ? Math.round(parseFloat(price) * 100) : null,
      currency: "EUR",
    });
    if (res.ok) {
      setDetailOrders((o) => [...(o ?? []), res.data]);
      setOrderForm({ platform: "", target_url: "", price: "" });
    } else {
      setSpotsErr(res.detail || "Could not add the order.");
    }
  }
  async function setOrderStatus(order: SpotOrder, status: string) {
    if (!orgId || !projectId || !detail) return;
    const res = await orgApi.updateSpotOrder(orgId, projectId, detail.id, order.id, {
      status,
      platform: order.platform,
      target_url: order.target_url,
      price_cents: order.price_cents,
      currency: order.currency,
    });
    if (res.ok) {
      setDetailOrders((o) => (o ?? []).map((x) => (x.id === order.id ? res.data : x)));
    }
  }
  async function removeOrder(orderId: string) {
    if (!orgId || !projectId || !detail) return;
    const res = await orgApi.deleteSpotOrder(orgId, projectId, detail.id, orderId);
    if (res.ok) {
      setDetailOrders((o) => (o ?? []).filter((x) => x.id !== orderId));
    }
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

  // The "Offers" cell: every marketplace that lists this spot, cheapest first
  // (price-compare). Falls back to the embedded single offer for API responses
  // that predate offers[].
  function renderOffers(s: SpotView) {
    const offers = s.offers ?? [];
    if (offers.length > 0) {
      return (
        <div className="sf-offers">
          {offers.map((o, i) => {
            const price = formatPrice(o.price_cents, o.currency);
            const follow =
              o.dofollow == null ? "" : o.dofollow ? " · dofollow" : " · nofollow";
            const title = `${o.source}${follow}`;
            return o.url ? (
              <a
                key={i}
                className="sf-buy"
                href={o.url}
                target="_blank"
                rel="noopener noreferrer"
                title={title}
              >
                {price} ↗
              </a>
            ) : (
              <span key={i} className="sf-buy" title={title}>
                {price}
              </span>
            );
          })}
        </div>
      );
    }
    if (s.buyable) {
      return s.marketplace_url ? (
        <a
          className="sf-buy"
          href={s.marketplace_url}
          target="_blank"
          rel="noopener noreferrer"
          title={`Buy on ${s.marketplace ?? "marketplace"}`}
        >
          {formatPrice(s.buy_price_cents, s.buy_currency)} ↗
        </a>
      ) : (
        <span className="sf-buy">{formatPrice(s.buy_price_cents, s.buy_currency)}</span>
      );
    }
    return <span className="muted">–</span>;
  }

  async function addGeoQuery() {
    if (!orgId || !projectId) return;
    const query = newGeoQuery.trim();
    if (!query) return;
    setGeoErr(null);
    setGeoBusy(true);
    const res = await orgApi.addGeoQuery(orgId, projectId, query);
    setGeoBusy(false);
    if (!res.ok) {
      setGeoErr(res.detail || "Could not add the query.");
      return;
    }
    setNewGeoQuery("");
    await loadGeo(orgId, projectId);
  }

  async function removeGeoQuery(id: string) {
    if (!orgId || !projectId) return;
    const res = await orgApi.deleteGeoQuery(orgId, projectId, id);
    if (res.ok) await loadGeo(orgId, projectId);
  }

  async function geoRefresh() {
    if (!orgId || !projectId) return;
    setGeoErr(null);
    setGeoBusy(true);
    const res = await orgApi.geoRefresh(orgId, projectId);
    setGeoBusy(false);
    if (!res.ok) {
      setGeoErr(res.detail || "Could not start the GEO refresh.");
      return;
    }
    // Captures run in the background; reload the citations shortly.
    const o = orgId;
    const p = projectId;
    setTimeout(() => void loadGeo(o, p), 4000);
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

  // Netlinking ranking: sort spots by current thematic performance (PTA) — this
  // IS the "Classement" — then apply the grade filter (all-on shows every spot).
  const rankedSpots = useMemo(() => {
    const all = spotsData?.spots ?? [];
    const allGradesOn = gradeFilter.size >= GRADE_ORDER.length;
    const filtered = allGradesOn
      ? all
      : all.filter((s) => gradeFilter.has(s.grade ?? "unrated"));
    return [...filtered].sort(
      (a, b) => (b.current_perf ?? 0) - (a.current_perf ?? 0),
    );
  }, [spotsData, gradeFilter]);
  // Does the current spot set carry thematic scores yet? (SERP run populated them.)
  const hasThematic = (spotsData?.spots ?? []).some(
    (s) => (s.ranked_keywords ?? 0) > 0,
  );

  // Selection summary: budget (sum of cheapest offers) + potential savings
  // (price spread on spots offered by more than one marketplace).
  const selectedSpots = (spotsData?.spots ?? []).filter((s) => selected.has(s.id));
  const selBudget = selectedSpots.reduce((sum, s) => sum + (s.buy_price_cents ?? 0), 0);
  const selSavings = selectedSpots.reduce((sum, s) => {
    const prices = (s.offers ?? [])
      .map((o) => o.price_cents)
      .filter((p): p is number => p != null);
    return prices.length > 1 ? sum + (Math.max(...prices) - Math.min(...prices)) : sum;
  }, 0);

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
          Enter your site URL — we read its topics, then find buyable link spots that rank on the
          same themes, scored and price-compared across your marketplaces. Buy, then monitor what you
          built.
        </p>
      </header>

      {me.organizations.length === 0 ? (
        <p className="muted">Create a workspace first.</p>
      ) : (
        <>
          <section className="panel sf-analyze">
            <div className="panel-head">
              <h2>Analyze a site</h2>
            </div>
            <p className="muted" style={{ marginBottom: 0 }}>
              Paste any site URL. We read its topics, then rank the link spots that match — scored
              (PTA/PTP) with marketplace prices compared. Buy, then monitor.
            </p>
            <form className="sf-analyze-form" onSubmit={analyze}>
              <div className="sf-analyze-field">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden="true">
                  <circle cx="12" cy="12" r="9" />
                  <path d="M3 12h18" />
                  <path d="M12 3a14 14 0 0 1 0 18a14 14 0 0 1 0-18" />
                </svg>
                <input
                  className="sf-analyze-input"
                  type="text"
                  inputMode="url"
                  placeholder="yoursite.com"
                  value={analyzeInput}
                  onChange={(e) => setAnalyzeInput(e.target.value)}
                  aria-label="Site URL to analyze"
                />
              </div>
              <select className="sf-analyze-market" aria-label="Market" value={locale} onChange={(e) => setLocale(e.target.value)}>
                <option value="fr">France · Français</option>
                <option value="en">English</option>
              </select>
              <button className="btn primary sf-analyze-btn" type="submit" disabled={analyzing || !analyzeInput.trim()}>
                {analyzing ? "Analyzing…" : "Analyze"}
              </button>
            </form>
            {pilot && (
              <div className="sf-status" role="status" style={{ marginTop: 12 }}>
                <span className="sf-spinner" aria-hidden="true" />
                {pilot}
              </div>
            )}
            {err && (
              <div className="auth-error" role="alert" style={{ marginTop: 12 }}>
                {err}
              </div>
            )}
          </section>

          {overview.length > 0 && (
            <section className="panel">
              <div className="panel-head">
                <h2>My analyses</h2>
                <span className="muted">{overview.length} site{overview.length === 1 ? "" : "s"}</span>
              </div>
              <div className="sf-cards">
                {overview.map((o) => (
                  <div key={o.id} className={`sf-card${o.id === projectId ? " active" : ""}`}>
                    <div className="sf-card-top">
                      <img
                        className="sf-card-fav"
                        src={`https://www.google.com/s2/favicons?sz=64&domain=${o.domain}`}
                        alt=""
                        width={20}
                        height={20}
                        loading="lazy"
                      />
                      <div style={{ minWidth: 0 }}>
                        <div className="sf-card-domain mono" title={o.domain}>{o.domain}</div>
                        <div className="sf-card-sub">
                          {o.spots} spot{o.spots === 1 ? "" : "s"}
                        </div>
                      </div>
                    </div>
                    {o.analyzing ? (
                      <button className="btn sf-card-cta" disabled>
                        &nbsp;
                        <span className="sf-card-progress">
                          <span className="sf-spinner" aria-hidden="true" /> Analyzing…
                        </span>
                      </button>
                    ) : (
                      <button className="btn primary" onClick={() => setProjectId(o.id)}>
                        See spots{o.spots > 0 ? ` (${o.spots})` : ""}
                      </button>
                    )}
                  </div>
                ))}
              </div>
            </section>
          )}

          {campList.length > 0 && (
            <section className="panel">
              <div className="panel-head">
                <h2>Purchase campaigns</h2>
                <span className="muted">{campList.length}</span>
              </div>
              <div className="bl-scroll">
                <table className="bl-table">
                  <thead>
                    <tr><th>Name</th><th>Status</th><th>Spots</th><th>Planned</th><th></th></tr>
                  </thead>
                  <tbody>
                    {campList.map((c) => (
                      <tr key={c.id}>
                        <td>{c.name}</td>
                        <td>{c.status}</td>
                        <td className="mono">{c.item_count}</td>
                        <td className="mono">{formatPrice(c.planned_cents, "EUR")}</td>
                        <td>
                          <button className="btn sm" onClick={() => void openCampaign(c.id)}>
                            View
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </section>
          )}

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
                    onClick={() => void discoverMarketplace()}
                    disabled={spotsBusy || busyRun || counts.kept === 0 || !marketplace}
                    title="Browse your link marketplaces for sites relevant to your kept keywords"
                  >
                    {searching ? "Discovering…" : "Discover from marketplaces"}
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
                    disabled={spotsBusy || busyRun || (spotsData?.spots.length ?? 0) === 0 || !marketplace}
                    title="Compare prices for these spots across your configured marketplaces"
                  >
                    {matching ? "Matching…" : "Compare prices"}
                  </button>
                  <button
                    className="btn sm ghost"
                    onClick={() => void search()}
                    disabled={spotsBusy || busyRun || counts.kept === 0 || !dataforseo}
                    title="Secondary: discover spots from Google rankings (SERP) instead of marketplaces"
                  >
                    {searching ? "Searching…" : "Find via Google (SERP)"}
                  </button>
                </div>
              </div>

              <p className="muted" style={{ marginBottom: 10 }}>
                We browse your configured link marketplaces for sites relevant to your kept
                keywords — the buyable spots, with their price and metrics. Enrich adds authority and
                real organic traffic; Compare prices lines up every marketplace&rsquo;s offer. Then
                buy, and add the link to monitoring. (Finding via Google rankings is available as a
                secondary source and uses DataForSEO.)
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
                  {hasThematic && (
                    <div className="sf-grade-chips">
                      {GRADE_ORDER.map((g) => (
                        <button
                          key={g}
                          type="button"
                          aria-pressed={gradeFilter.has(g)}
                          className={`sf-grade-chip g-${g}`}
                          title={`Show ${GRADE_LABEL[g]} spots`}
                          onClick={() =>
                            setGradeFilter((prev) => {
                              const next = new Set(prev);
                              if (next.has(g)) next.delete(g);
                              else next.add(g);
                              return next;
                            })
                          }
                        >
                          {GRADE_LABEL[g]}
                        </button>
                      ))}
                      <span className="sf-grade-count">{rankedSpots.length} shown</span>
                    </div>
                  )}
                  <div className="bl-scroll">
                  <table className="bl-table sf-spots-table">
                    <thead>
                      <tr>
                        <th style={{ width: 28 }}>
                          <input
                            type="checkbox"
                            aria-label="Select all shown spots"
                            checked={rankedSpots.length > 0 && rankedSpots.every((s) => selected.has(s.id))}
                            onChange={(e) =>
                              setSelected(
                                e.target.checked
                                  ? new Set(rankedSpots.map((s) => s.id))
                                  : new Set(),
                              )
                            }
                          />
                        </th>
                        {hasThematic && <th title="Rank by current thematic performance">#</th>}
                        {hasThematic && (
                          <th title="Current thematic performance — how much it already weighs across your whole topic">PTA</th>
                        )}
                        {hasThematic && (
                          <th title="Potential thematic performance — quality of its positions where it ranks">PTP</th>
                        )}
                        {hasThematic && <th title="Price ÷ performance grade, relative to this list">Grade</th>}
                        <th title="LinkPulz quality score (0–100)">Score</th>
                        <th>Domain</th>
                        <th>Keywords</th>
                        <th>Best</th>
                        <th title="Domain authority (DataForSEO rank 0–1000)">DR</th>
                        <th title="Estimated monthly organic traffic">Traffic</th>
                        <th title="Backlink spam score (lower is better)">Spam</th>
                        <th title="Placements available, cheapest first">Offers</th>
                        <th>Example</th>
                        <th></th>
                        {campaignsBeta && <th></th>}
                      </tr>
                    </thead>
                    <tbody>
                      {rankedSpots.map((s, i) => (
                        <tr key={s.id}>
                          <td>
                            <input
                              type="checkbox"
                              aria-label={`Select ${s.domain}`}
                              checked={selected.has(s.id)}
                              onChange={() => toggleSelected(s.id)}
                            />
                          </td>
                          {hasThematic && <td className="sf-rank">{i + 1}</td>}
                          {hasThematic && <td><ScoreDonut v={s.current_perf} size={34} /></td>}
                          {hasThematic && <td><ScoreDonut v={s.potential_perf} size={34} /></td>}
                          {hasThematic && <td><GradeBadge grade={s.grade} /></td>}
                          <td>
                            {s.quality_score != null ? (
                              <span className="sf-score">{Math.round(s.quality_score)}</span>
                            ) : (
                              <span className="muted">–</span>
                            )}
                          </td>
                          <td className="mono">
                            <button
                              type="button"
                              className="sf-domain-btn"
                              title="Open spot detail"
                              onClick={() => openDetail(s)}
                            >
                              {s.domain}
                            </button>
                          </td>
                          <td>
                            <b>{s.keyword_count}</b>
                          </td>
                          <td>{s.best_position ?? "-"}</td>
                          <td>{s.domain_rank ?? "–"}</td>
                          <td>{s.organic_traffic != null ? s.organic_traffic.toLocaleString() : "–"}</td>
                          <td>{s.spam_score ?? "–"}</td>
                          <td>{renderOffers(s)}</td>
                          <td className="sf-example">
                            {s.example_url ? (
                              <a href={s.example_url} target="_blank" rel="noopener noreferrer" title={s.example_url}>
                                {s.example_keyword ?? "page"} ↗
                              </a>
                            ) : (
                              (s.example_keyword ?? "-")
                            )}
                          </td>
                          <td>
                            <button
                              className="btn sm"
                              title="Bought this link? Add it to monitoring (set the article URL once it's live)"
                              onClick={() => void monitorSpot(s.id)}
                            >
                              Monitor
                            </button>
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
                      {rankedSpots.length === 0 && (
                        <tr>
                          <td colSpan={99} className="muted">
                            No spots match the selected grades.
                          </td>
                        </tr>
                      )}
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

          <section className="panel sf-geo">
            <h2>
              GEO — cited by AI <span className="sf-beta">BETA</span>
            </h2>
            <p className="muted" style={{ marginBottom: 10 }}>
              Track which sources AI assistants cite for your queries (Google AI Overviews), and
              see which cited domains you can actually place a link on. Each refresh uses DataForSEO
              and counts against your plan&rsquo;s GEO allowance.
            </p>
            {geoErr && (
              <div className="auth-error" role="alert" style={{ marginBottom: 10 }}>
                {geoErr}
              </div>
            )}
            <form
              className="sf-add"
              onSubmit={(e) => {
                e.preventDefault();
                void addGeoQuery();
              }}
            >
              <input
                aria-label="Add a GEO query"
                value={newGeoQuery}
                onChange={(e) => setNewGeoQuery(e.target.value)}
                placeholder="a question your buyers ask an AI…"
              />
              <button className="btn sm" type="submit" disabled={geoBusy || !newGeoQuery.trim()}>
                Add query
              </button>
            </form>
            {geoQueries.length > 0 && (
              <ul className="sf-list" style={{ marginTop: 10 }}>
                {geoQueries.map((q) => (
                  <li key={q.id} className="sf-item">
                    <div className="sf-item-main">
                      <span className="mono">{q.query}</span> <span className="sf-tag">{q.locale}</span>
                    </div>
                    <button
                      className="btn sm ghost"
                      onClick={() => void removeGeoQuery(q.id)}
                      title="Stop tracking this query"
                    >
                      Remove
                    </button>
                  </li>
                ))}
              </ul>
            )}
            <div className="sf-spots-head" style={{ marginTop: 12 }}>
              <span className="muted">
                {geoQueries.length} quer{geoQueries.length === 1 ? "y" : "ies"} tracked
                {geoCites ? ` · ${geoCites.domains.length} cited domain${geoCites.domains.length === 1 ? "" : "s"}` : ""}
              </span>
              <button
                className="btn primary"
                onClick={() => void geoRefresh()}
                disabled={geoBusy || geoQueries.length === 0 || !dataforseo}
              >
                {geoBusy ? "Refreshing…" : "Refresh AI answers"}
              </button>
            </div>
            {!dataforseo && (
              <div className="auth-error" role="alert" style={{ marginTop: 10 }}>
                DataForSEO is not configured on this deployment, so GEO capture is disabled.
              </div>
            )}
            {geoCites && geoCites.domains.length > 0 ? (
              <div className="bl-scroll" style={{ marginTop: 10 }}>
                <table className="bl-table sf-spots-table">
                  <thead>
                    <tr>
                      <th title="GEO opportunity score (0–100)">GEO</th>
                      <th>Domain</th>
                      <th title="Times cited across your tracked queries">Cited</th>
                      <th title="Best position among an answer's cited sources">Best rank</th>
                      <th title="Distinct AI engines that cite it">Engines</th>
                      <th title="Is there a spot or marketplace offer to place a link?">Placeable</th>
                    </tr>
                  </thead>
                  <tbody>
                    {geoCites.domains.map((d) => (
                      <tr key={d.domain}>
                        <td>
                          <span className="sf-score">{Math.round(d.score)}</span>
                        </td>
                        <td className="mono">{d.domain}</td>
                        <td>
                          <b>{d.citation_count}</b>
                        </td>
                        <td>{d.best_rank ?? "–"}</td>
                        <td>{d.engine_count}</td>
                        <td>
                          {d.placeable ? (
                            <span className="sf-buy">placeable</span>
                          ) : (
                            <span className="muted">–</span>
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <p className="muted" style={{ marginTop: 10 }}>
                No AI citations captured yet. Add the queries your buyers ask, then hit Refresh.
              </p>
            )}
          </section>
        </>
      )}

      {detail && (
        <div className="sf-drawer-backdrop" onClick={() => setDetail(null)}>
          <aside
            onClick={(e) => e.stopPropagation()}
            role="dialog"
            aria-modal="true"
            aria-label={`Spot detail for ${detail.domain}`}
            className="sf-drawer"
          >
            <div className="sf-drawer-head">
              <h2 className="mono" style={{ margin: 0, fontSize: 18 }}>{detail.domain}</h2>
              <button className="btn sm" onClick={() => setDetail(null)}>Close</button>
            </div>
            <div style={{ marginTop: 8, display: "flex", gap: 10, alignItems: "center" }}>
              <GradeBadge grade={detail.grade} />
              {detail.marketplace_url && (
                <a href={detail.marketplace_url} target="_blank" rel="noopener noreferrer">Buy ↗</a>
              )}
            </div>

            <div className="sf-drawer-scores">
              <div className="sf-drawer-score">
                <ScoreDonut v={detail.current_perf} size={54} />
                <div>
                  <div className="t">Current thematic perf</div>
                  <div className="d">what it already weighs across your topic</div>
                </div>
              </div>
              <div className="sf-drawer-score">
                <ScoreDonut v={detail.potential_perf} size={54} />
                <div>
                  <div className="t">Potential thematic perf</div>
                  <div className="d">quality of its positions where it ranks</div>
                </div>
              </div>
            </div>

            <div className="sf-drawer-metrics">
              <Stat label="DR" v={detail.domain_rank} />
              <Stat label="Ref. domains" v={detail.referring_domains} />
              <Stat label="Traffic/mo" v={detail.organic_traffic} />
              <Stat label="Spam" v={detail.spam_score} />
              <Stat label="Ranked kw" v={detail.ranked_keywords} />
            </div>

            <div className="sf-tabs" style={{ marginTop: 18 }}>
              <TabBtn active={detailTab === "keywords"} onClick={() => setDetailTab("keywords")}>
                Keywords positioned
              </TabBtn>
              <TabBtn active={detailTab === "prices"} onClick={() => setDetailTab("prices")}>
                Prices per platform
              </TabBtn>
              <TabBtn active={detailTab === "orders"} onClick={() => setDetailTab("orders")}>
                Order tracking{detailOrders && detailOrders.length > 0 ? ` (${detailOrders.length})` : ""}
              </TabBtn>
            </div>

            {detailTab === "keywords" ? (
              detailKw == null ? (
                <p className="muted" style={{ marginTop: 12 }}>Loading…</p>
              ) : detailKw.length === 0 ? (
                <p className="muted" style={{ marginTop: 12 }}>
                  No ranked keywords captured for this spot yet — run “Find via Google (SERP)”.
                </p>
              ) : (
                <table className="bl-table" style={{ marginTop: 12 }}>
                  <thead>
                    <tr><th>Keyword</th><th>Pos</th><th>Vol/mo</th><th>Page</th></tr>
                  </thead>
                  <tbody>
                    {detailKw.map((k, i) => (
                      <tr key={i}>
                        <td>{k.keyword}</td>
                        <td className="mono">{k.position}</td>
                        <td className="mono">{k.search_volume != null ? k.search_volume.toLocaleString() : "–"}</td>
                        <td className="sf-example">
                          {k.page_url ? (
                            <a href={k.page_url} target="_blank" rel="noopener noreferrer" title={k.page_url}>page ↗</a>
                          ) : "–"}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )
            ) : detailTab === "prices" ? (
              (detail.offers?.length ?? 0) === 0 ? (
                <p className="muted" style={{ marginTop: 12 }}>
                  No marketplace offers for this spot yet — run “Compare prices”.
                </p>
              ) : (
                <table className="bl-table" style={{ marginTop: 12 }}>
                  <thead>
                    <tr><th>Platform</th><th>Price</th><th></th></tr>
                  </thead>
                  <tbody>
                    {detail.offers!.map((o, i) => (
                      <tr key={i}>
                        <td>{o.source}</td>
                        <td className="mono">
                          {o.price_cents != null ? `${(o.price_cents / 100).toLocaleString()} ${o.currency ?? "EUR"}` : "n/a"}
                        </td>
                        <td>{o.url ? <a href={o.url} target="_blank" rel="noopener noreferrer">Buy ↗</a> : null}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )
            ) : (
              <div style={{ marginTop: 12 }}>
                <div style={{ display: "flex", gap: 6, flexWrap: "wrap", alignItems: "center", marginBottom: 12 }}>
                  <input
                    placeholder="Platform"
                    value={orderForm.platform}
                    onChange={(e) => setOrderForm((f) => ({ ...f, platform: e.target.value }))}
                    style={{ width: 120, padding: "6px 8px", borderRadius: 8, border: "1px solid var(--border)", background: "var(--bg)", color: "var(--fg)" }}
                  />
                  <input
                    placeholder="Target URL"
                    value={orderForm.target_url}
                    onChange={(e) => setOrderForm((f) => ({ ...f, target_url: e.target.value }))}
                    style={{ flex: "1 1 160px", padding: "6px 8px", borderRadius: 8, border: "1px solid var(--border)", background: "var(--bg)", color: "var(--fg)" }}
                  />
                  <input
                    placeholder="€"
                    inputMode="decimal"
                    value={orderForm.price}
                    onChange={(e) => setOrderForm((f) => ({ ...f, price: e.target.value }))}
                    style={{ width: 70, padding: "6px 8px", borderRadius: 8, border: "1px solid var(--border)", background: "var(--bg)", color: "var(--fg)" }}
                  />
                  <button className="btn sm primary" onClick={() => void addOrder()}>Add</button>
                </div>
                {detailOrders == null ? (
                  <p className="muted">Loading…</p>
                ) : detailOrders.length === 0 ? (
                  <p className="muted">No orders yet. Track what you ordered, where and for how much.</p>
                ) : (
                  <table className="bl-table">
                    <thead>
                      <tr><th>Status</th><th>Platform</th><th>Target</th><th>Price</th><th></th></tr>
                    </thead>
                    <tbody>
                      {detailOrders.map((o) => (
                        <tr key={o.id}>
                          <td>
                            <select
                              value={o.status}
                              onChange={(e) => void setOrderStatus(o, e.target.value)}
                              aria-label="Order status"
                            >
                              <option value="todo">To do</option>
                              <option value="in_progress">In progress</option>
                              <option value="done">Done</option>
                              <option value="problem">Problem</option>
                            </select>
                          </td>
                          <td>{o.platform ?? "—"}</td>
                          <td className="sf-example">
                            {o.target_url ? (
                              <a href={o.target_url} target="_blank" rel="noopener noreferrer" title={o.target_url}>link ↗</a>
                            ) : "—"}
                          </td>
                          <td className="mono">
                            {o.price_cents != null ? `${(o.price_cents / 100).toLocaleString()} ${o.currency ?? "EUR"}` : "—"}
                          </td>
                          <td>
                            <button className="btn sm danger" title="Delete order" onClick={() => void removeOrder(o.id)}>×</button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </div>
            )}

            <div style={{ marginTop: 18 }}>
              <button className="btn" onClick={() => void monitorSpot(detail.id)}>Monitor this link</button>
            </div>
          </aside>
        </div>
      )}
      {campDetail && (
        <div
          onClick={() => setCampDetail(null)}
          style={{ position: "fixed", inset: 0, background: "rgba(5,10,25,.45)", zIndex: 60, display: "flex", justifyContent: "flex-end" }}
        >
          <aside
            onClick={(e) => e.stopPropagation()}
            role="dialog"
            aria-modal="true"
            aria-label={`Campaign ${campDetail.campaign.name}`}
            style={{ width: "min(620px, 100%)", height: "100%", overflowY: "auto", background: "var(--surface)", borderLeft: "1px solid var(--border)", padding: 22 }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8 }}>
              <h2 style={{ margin: 0, fontSize: 18 }}>{campDetail.campaign.name}</h2>
              <div style={{ display: "flex", gap: 8 }}>
                <button className="btn sm danger" onClick={() => void deleteCampaign()}>Delete</button>
                <button className="btn sm" onClick={() => setCampDetail(null)}>Close</button>
              </div>
            </div>
            <div style={{ display: "flex", gap: 16, marginTop: 8, flexWrap: "wrap", alignItems: "center" }}>
              <label className="muted" style={{ display: "flex", gap: 6, alignItems: "center" }}>
                Status
                <select value={campDetail.campaign.status} onChange={(e) => void setCampaignStatus(e.target.value)}>
                  <option value="draft">Draft</option>
                  <option value="ready">Ready</option>
                  <option value="ordering">Ordering</option>
                  <option value="published">Published</option>
                  <option value="done">Done</option>
                </select>
              </label>
              <span className="muted">{campDetail.campaign.item_count} spots</span>
              <span className="muted">
                Planned <b style={{ color: "var(--text)" }}>{formatPrice(campDetail.campaign.planned_cents, "EUR")}</b>
              </span>
            </div>
            <table className="bl-table" style={{ marginTop: 14 }}>
              <thead>
                <tr><th>Domain</th><th>Price</th><th>Marketplace</th><th>Status</th><th></th></tr>
              </thead>
              <tbody>
                {campDetail.items.map((it) => (
                  <tr key={it.id}>
                    <td className="mono">{it.domain}</td>
                    <td className="mono">{it.price_cents != null ? formatPrice(it.price_cents, it.currency ?? "EUR") : "—"}</td>
                    <td>{it.source_slug ?? "—"}</td>
                    <td>
                      <select value={it.status} onChange={(e) => void setItemStatus(it, e.target.value)} aria-label="Item status">
                        <option value="todo">To do</option>
                        <option value="ordered">Ordered</option>
                        <option value="published">Published</option>
                        <option value="problem">Problem</option>
                      </select>
                    </td>
                    <td>
                      <button className="btn sm danger" title="Remove from campaign" onClick={() => void removeCampaignItem(it.id)}>×</button>
                    </td>
                  </tr>
                ))}
                {campDetail.items.length === 0 && (
                  <tr><td colSpan={5} className="muted">No spots in this campaign.</td></tr>
                )}
              </tbody>
            </table>
            <p className="muted" style={{ fontSize: 11, marginTop: 10 }}>
              Track each spot's order status here. Direct API ordering (LinkAvista…) comes next.
            </p>
          </aside>
        </div>
      )}

      {selected.size > 0 && (
        <div
          style={{
            position: "fixed",
            left: 0,
            right: 0,
            bottom: 0,
            zIndex: 40,
            background: "var(--surface)",
            borderTop: "1px solid var(--border)",
            boxShadow: "0 -2px 14px rgba(5,10,25,.08)",
            padding: "12px 20px",
            display: "flex",
            gap: 16,
            alignItems: "center",
            flexWrap: "wrap",
          }}
        >
          <strong>
            {selected.size} spot{selected.size === 1 ? "" : "s"} selected
          </strong>
          <span className="muted">
            Budget <b style={{ color: "var(--text)" }}>{formatPrice(selBudget, "EUR")}</b>
          </span>
          {selSavings > 0 && (
            <span className="muted">
              Potential savings{" "}
              <b style={{ color: "var(--live)" }}>{formatPrice(selSavings, "EUR")}</b>
            </span>
          )}
          <div style={{ marginLeft: "auto", display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
            {campaignsBeta && campaigns.length > 0 && (
              <>
                <select
                  value={campaignId}
                  onChange={(e) => setCampaignId(e.target.value)}
                  aria-label="Target campaign"
                >
                  <option value="">Campaign…</option>
                  {campaigns.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
                </select>
                <button
                  className="btn sm"
                  disabled={selBusy || !campaignId}
                  onClick={() => void addSelectedToCampaign([...selected])}
                >
                  Add to campaign
                </button>
              </>
            )}
            <button
              className="btn sm primary"
              disabled={selBusy}
              onClick={() => createCampaignFromSelection([...selected])}
            >
              {selBusy ? "Working…" : "Create campaign"}
            </button>
            <button
              className="btn sm"
              disabled={selBusy}
              onClick={() => void monitorSelected([...selected])}
            >
              Monitor selected
            </button>
            <button className="btn sm ghost" disabled={selBusy} onClick={() => setSelected(new Set())}>
              Clear
            </button>
          </div>
        </div>
      )}

      {campModal && (
        <div
          className="dom-modal-backdrop"
          onClick={() => {
            if (!selBusy) setCampModal(null);
          }}
        >
          <div
            className="dom-modal"
            role="dialog"
            aria-modal="true"
            aria-label="Name this purchase campaign"
            onClick={(e) => e.stopPropagation()}
          >
            <h2>New purchase campaign</h2>
            <form
              className="ops-form"
              onSubmit={(e) => {
                e.preventDefault();
                void confirmCreateCampaign();
              }}
            >
              {spotsErr && (
                <div className="auth-error" role="alert">
                  {spotsErr}
                </div>
              )}
              <label className="auth-field">
                <span>Campaign name</span>
                <input
                  value={campModal.name}
                  onChange={(e) =>
                    setCampModal((m) => (m ? { ...m, name: e.target.value } : m))
                  }
                  autoFocus
                  required
                  maxLength={120}
                />
              </label>
              <p className="muted" style={{ margin: 0, fontSize: 13 }}>
                {campModal.ids.length} spot{campModal.ids.length === 1 ? "" : "s"} will be added.
              </p>
              <div className="dom-modal-actions">
                <button
                  type="button"
                  className="btn"
                  disabled={selBusy}
                  onClick={() => setCampModal(null)}
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="btn primary"
                  disabled={selBusy || !campModal.name.trim()}
                >
                  {selBusy ? "Creating…" : "Create campaign"}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </main>
  );
}
