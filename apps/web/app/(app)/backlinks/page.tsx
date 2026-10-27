"use client";

import { useCallback, useEffect, useRef, useState, type CSSProperties, type FormEvent, type ReactNode } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { api, orgApi, backlinksExportUrl, type AnchorsView, type AttentionItemView, type BacklinkView, type ClientView, type DashboardResponse, type EventView, type ImpactView, type MeResponse, type OccurrenceView, type OverviewResponse, type ProjectView, type TagView } from "@/lib/api";
import { BeforeAfterChart } from "@/components/BeforeAfterChart";
import { GrowthChart } from "@/components/GrowthChart";
import { AnchorDistribution } from "@/components/AnchorDistribution";
import { selectedOrgId } from "@/lib/workspace";
import { statusColor, statusLabel } from "@/lib/status";
import { ACQUISITION, ATTRIBUTE, PLACEMENT, FORMAT, classLabel } from "@/lib/classification";
import {
  MONITORING_LEVEL,
  CHECK_FREQUENCY,
  COMMERCIAL_MODEL,
  monitoringLabel,
  LEVEL_BADGE,
} from "@/lib/monitoring";
import { toast } from "../toast";
import { Icon } from "../icons";
import { useDrawerFocus } from "@/lib/useDrawerFocus";
import "@/app/auth.css";
import "../dashboard/dashboard.css";
import "../dashboard/overview.css";
import "./impact.css";
import "./backlinks.css";

export default function BacklinksPage() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [summary, setSummary] = useState<DashboardResponse | null>(null);
  const [projects, setProjects] = useState<ProjectView[]>([]);
  const [projectId, setProjectId] = useState<string | null>(null);
  const [backlinks, setBacklinks] = useState<BacklinkView[]>([]);
  const [selected, setSelected] = useState<BacklinkView | null>(null);
  const [events, setEvents] = useState<EventView[] | null>(null);
  const [impact, setImpact] = useState<ImpactView | null>(null);
  const [impactBusy, setImpactBusy] = useState(false);
  const [occurrences, setOccurrences] = useState<OccurrenceView[]>([]);
  const [tier2, setTier2] = useState<BacklinkView[]>([]);
  const [t2csv, setT2csv] = useState("");
  const [t2msg, setT2msg] = useState<string | null>(null);
  const [warranties, setWarranties] = useState<BacklinkView[]>([]);
  const [clients, setClients] = useState<ClientView[]>([]);
  const [orgTags, setOrgTags] = useState<TagView[]>([]);
  const [blTags, setBlTags] = useState<TagView[]>([]);
  const [newTag, setNewTag] = useState("");
  const [classBusy, setClassBusy] = useState(false);
  const [statusFilter, setStatusFilter] = useState("");
  const [clientFilter, setClientFilter] = useState("");
  const [searchQ, setSearchQ] = useState("");
  const [domainFilter, setDomainFilter] = useState("");
  const [tagFilter, setTagFilter] = useState("");
  // Classification filters (migration 0034); "" = all.
  const [acqFilter, setAcqFilter] = useState("");
  const [attrFilter, setAttrFilter] = useState("");
  const [placeFilter, setPlaceFilter] = useState("");
  const [fmtFilter, setFmtFilter] = useState("");
  // Monitoring view (segmented tab) + multi-select for bulk actions.
  const [monitorFilter, setMonitorFilter] = useState(""); // "" | protected | tracked | inventory_only
  const [lowValue, setLowValue] = useState(false); // footprint-flagged spam/self-built links
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [bulkBusy, setBulkBusy] = useState(false);
  const [reloadTick, setReloadTick] = useState(0);
  const [anchors, setAnchors] = useState<AnchorsView | null>(null);
  // Org-wide command-center data (real series + pre-prioritized attention), the
  // same source the /dashboard uses. Powers the KPI row, the evolution chart and
  // the actionable insights at the top of this page.
  const [overview, setOverview] = useState<OverviewResponse | null>(null);
  const [periodDays, setPeriodDays] = useState(0); // 0 = all time

  // Preset the view from an Overview/nav deep-link: ?filter=protected|lost.
  // Deep-link drill-down: KPI tiles, Overview insights and external links can
  // land straight on a filtered view via ?filter= / ?status= / ?monitoring=.
  useEffect(() => {
    const f = searchParams.get("filter");
    if (f === "protected") setMonitorFilter("protected");
    else if (f === "lost") setStatusFilter("lost");
    else if (f && STATUSES.includes(f)) setStatusFilter(f);
    const st = searchParams.get("status");
    if (st && STATUSES.includes(st)) setStatusFilter(st);
    const mon = searchParams.get("monitoring");
    if (mon && ["protected", "tracked", "inventory_only"].includes(mon)) setMonitorFilter(mon);
  }, [searchParams]);

  // Overview "Review" deep-link: ?focus=<backlink id> opens that link's detail
  // drawer straight away. The ref guards against re-opening on re-render or after
  // the user closes the drawer.
  const focusedRef = useRef<string | null>(null);
  useEffect(() => {
    const focus = searchParams.get("focus");
    if (!focus || !orgId || focusedRef.current === focus) return;
    focusedRef.current = focus;
    void orgApi.getBacklink(orgId, focus).then((res) => {
      if (res.ok) void openDetail(res.data);
    });
    // openDetail is a stable (hoisted) component fn; focusedRef prevents re-runs.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [searchParams, orgId]);
  const [page, setPage] = useState(0);
  const [hasNext, setHasNext] = useState(false);
  const [autoTagBusy, setAutoTagBusy] = useState(false);
  const [pageSize, setPageSize] = useState(50);
  const [loading, setLoading] = useState(true);
  const [wsName, setWsName] = useState("");
  const [wsBusy, setWsBusy] = useState(false);
  const [wsErr, setWsErr] = useState<string | null>(null);
  const [showAdd, setShowAdd] = useState(false);
  const [addBusy, setAddBusy] = useState(false);
  const [addErr, setAddErr] = useState<string | null>(null);

  // Close any open drawer (add / notifications / detail) with Escape.
  useEffect(() => {
    if (!showAdd && !selected) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setShowAdd(false);
        setSelected(null);
      }
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [showAdd, selected]);
  // Focus the open drawer and restore focus to the opener on close (a11y).
  const addDrawerRef = useRef<HTMLElement | null>(null);
  const detailDrawerRef = useRef<HTMLElement | null>(null);
  useDrawerFocus(showAdd, addDrawerRef);
  useDrawerFocus(!!selected, detailDrawerRef);
  const [addForm, setAddForm] = useState({
    project_id: "",
    source_url: "",
    target_url: "",
    expected_rel_type: "dofollow",
    expected_anchor_text: "",
    cost: "",
    currency: "",
  });
  // Activation UX: monitor new links by default (the "Monitor these links" box),
  // and a bulk-paste path (the primary way an SEO adds many links at once).
  const [monitorNew, setMonitorNew] = useState(true);
  const [bulkText, setBulkText] = useState("");
  const [bulkAddBusy, setBulkAddBusy] = useState(false);
  const [showSingle, setShowSingle] = useState(false);

  const STATUSES = ["live", "suspected_lost", "lost", "changed", "nofollow", "redirected", "source_error", "target_error", "blocked", "pending"];

  useEffect(() => {
    api.me().then((res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      setMe(res.data);
      const first = selectedOrgId(res.data.organizations);
      setOrgId(first);
      setLoading(false);
    });
  }, [router]);

  const loadOrg = useCallback(
    async (org: string) => {
      const [dash, projs, warr, ov] = await Promise.all([
        orgApi.dashboard(org),
        orgApi.projects(org),
        orgApi.expiringWarranties(org),
        orgApi.overview(org),
      ]);
      if (dash.ok) setSummary(dash.data);
      if (ov.ok) setOverview(ov.data);
      if (projs.ok) {
        setProjects(projs.data);
        const p = projs.data[0]?.id ?? null;
        setProjectId(p);
      }
      if (warr.ok) setWarranties(warr.data);
      const [tags, cls] = await Promise.all([orgApi.tags(org), orgApi.clients(org)]);
      if (tags.ok) setOrgTags(tags.data);
      if (cls.ok) setClients(cls.data);
    },
    [],
  );

  useEffect(() => {
    if (orgId) void loadOrg(orgId);
  }, [orgId, loadOrg]);

  useEffect(() => {
    // A client filter shows that client's backlinks across all its projects;
    // otherwise the view is scoped to the selected project.
    if (!orgId || (!clientFilter && !projectId)) {
      setBacklinks([]);
      setHasNext(false);
      return;
    }
    const t = setTimeout(() => {
      orgApi
        .searchBacklinks(orgId, {
          projectId: clientFilter ? undefined : projectId || undefined,
          clientId: clientFilter || undefined,
          status: statusFilter || undefined,
          q: searchQ.trim() || undefined,
          tagId: tagFilter || undefined,
          days: periodDays || undefined,
          acquisition: acqFilter || undefined,
          attribute: attrFilter || undefined,
          placement: placeFilter || undefined,
          format: fmtFilter || undefined,
          monitoringLevel: monitorFilter || undefined,
          lowValue: lowValue || undefined,
          limit: pageSize,
          offset: page * pageSize,
        })
        .then((res) => {
          if (res.ok) {
            setBacklinks(res.data);
            setHasNext(res.data.length === pageSize);
            setSelectedIds(new Set()); // clear selection on any reload
          }
        });
    }, 200); // debounce the search box
    return () => clearTimeout(t);
  }, [orgId, projectId, clientFilter, statusFilter, searchQ, tagFilter, periodDays, acqFilter, attrFilter, placeFilter, fmtFilter, monitorFilter, lowValue, reloadTick, page, pageSize]);

  // Any filter (or page-size) change resets to the first page. (The load effect's
  // debounce cleanup cancels the stale-page fetch, so this doesn't double-request.)
  useEffect(() => {
    setPage(0);
  }, [orgId, projectId, clientFilter, statusFilter, searchQ, tagFilter, periodDays, acqFilter, attrFilter, placeFilter, fmtFilter, monitorFilter, lowValue, pageSize]);

  // Drop a referring-domain filter that no longer matches the reloaded list.
  useEffect(() => {
    if (domainFilter && !backlinks.some((b) => b.source_domain === domainFilter)) {
      setDomainFilter("");
    }
  }, [backlinks, domainFilter]);

  // Reload the dashboard stats + illustrations when the period changes.
  useEffect(() => {
    if (!orgId) return;
    orgApi.dashboard(orgId, periodDays || undefined).then((r) => {
      if (r.ok) setSummary(r.data);
    });
  }, [orgId, periodDays]);

  // Anchor text distribution: per-project, recomputed when the project changes
  // or links are added/checked (reloadTick). Read-only, independent of filters.
  useEffect(() => {
    if (!orgId || !projectId) {
      setAnchors(null);
      return;
    }
    let live = true;
    orgApi.anchors(orgId, projectId).then((r) => {
      if (live) setAnchors(r.ok ? r.data : null);
    });
    return () => {
      live = false;
    };
  }, [orgId, projectId, reloadTick]);

  async function runAutoTag() {
    if (!orgId || !projectId) return;
    if (
      !confirm(
        "Auto-tag this project's backlinks by footprint?\n\n" +
          "Adds a category tag to each link: comment (ninja linking), pbn, forum-profile, self-built, or earned. " +
          "Reversible, you can retag or remove tags afterward.",
      )
    )
      return;
    setAutoTagBusy(true);
    const res = await orgApi.autoTagBacklinks(orgId, projectId);
    setAutoTagBusy(false);
    if (!res.ok) {
      toast(`Auto-tag failed: ${res.detail}`, "error");
      return;
    }
    const c = res.data.counts;
    const summary = Object.keys(c)
      .sort((a, b) => (c[b] ?? 0) - (c[a] ?? 0))
      .map((k) => `${k} ${c[k]}`)
      .join(" · ");
    toast(`Tagged ${res.data.total} backlinks, ${summary}`, "success");
    // Refresh the org tag list + the illustrations so the new tags show up.
    const [tags, dash] = await Promise.all([orgApi.tags(orgId), orgApi.dashboard(orgId, periodDays || undefined)]);
    if (tags.ok) setOrgTags(tags.data);
    if (dash.ok) setSummary(dash.data);
  }

  // Set one classification dimension on the open backlink. Empty value clears it
  // (back to unclassified / auto for the attribute). Updates the drawer + the row.
  async function setClass(
    field: "acquisition" | "link_attribute" | "placement" | "link_format",
    value: string,
  ) {
    if (!orgId || !selected) return;
    const body: {
      acquisition?: string | null;
      link_attribute?: string | null;
      placement?: string | null;
      link_format?: string | null;
    } = {};
    body[field] = value || null;
    setClassBusy(true);
    const res = await orgApi.updateBacklink(orgId, selected.id, body);
    setClassBusy(false);
    if (!res.ok) {
      toast(`Update failed: ${res.detail}`, "error");
      return;
    }
    setSelected(res.data);
    setBacklinks((bs) => bs.map((x) => (x.id === res.data.id ? res.data : x)));
  }

  // Patch the open backlink (monitoring/economic fields) and reflect it in-place.
  async function patchBacklink(body: Parameters<typeof orgApi.updateBacklink>[2]) {
    if (!orgId || !selected) return;
    const res = await orgApi.updateBacklink(orgId, selected.id, body);
    if (!res.ok) {
      toast(`Update failed: ${res.detail}`, "error");
      return;
    }
    setSelected(res.data);
    setBacklinks((bs) => bs.map((x) => (x.id === res.data.id ? res.data : x)));
  }

  // Cost + currency are edited together (currency is required when a cost is
  // set) via these refs, saved on blur of either field.
  const costRef = useRef<HTMLInputElement>(null);
  const currencyRef = useRef<HTMLInputElement>(null);
  function saveCostCurrency() {
    if (!selected) return;
    const costV = costRef.current?.value.trim() ?? "";
    const curV = (currencyRef.current?.value.trim() ?? "").toUpperCase();
    const costNum = costV ? Number(costV) : null;
    if (costNum != null && (Number.isNaN(costNum) || costNum < 0)) return;
    const origCost = selected.cost != null ? Number(selected.cost) : null;
    if (costNum === origCost && curV === (selected.currency ?? "")) return; // unchanged
    if (costNum != null && !curV) {
      toast("Add a 3-letter currency (e.g. EUR) for the cost.", "error");
      return;
    }
    void patchBacklink({ cost: costNum, currency: costNum != null ? curV : null });
  }

  function toggleSelect(id: string) {
    setSelectedIds((s) => {
      const n = new Set(s);
      if (n.has(id)) n.delete(id);
      else n.add(id);
      return n;
    });
  }

  // Bulk monitoring change over the current selection, then reload.
  async function bulkMonitor(body: {
    monitoring_enabled?: boolean;
    monitoring_level?: string;
    check_frequency?: string;
  }) {
    if (!orgId || selectedIds.size === 0) return;
    setBulkBusy(true);
    const res = await orgApi.bulkMonitoring(orgId, [...selectedIds], body);
    setBulkBusy(false);
    if (!res.ok) {
      toast(`Bulk update failed: ${res.detail}`, "error");
      return;
    }
    toast(`Updated ${res.data.updated} backlink${res.data.updated === 1 ? "" : "s"}`, "success");
    setReloadTick((t) => t + 1); // reload (also clears the selection)
  }

  // Bulk archive the current selection (reversible) — for clearing a triaged
  // spam/low-value pile out of the active inventory.
  async function bulkArchive() {
    if (!orgId || selectedIds.size === 0) return;
    if (
      !window.confirm(
        `Archive ${selectedIds.size} backlink${selectedIds.size === 1 ? "" : "s"}? ` +
          "They leave your active inventory and stop being checked. You can restore them later.",
      )
    )
      return;
    setBulkBusy(true);
    const res = await orgApi.bulkArchiveBacklinks(orgId, [...selectedIds]);
    setBulkBusy(false);
    if (!res.ok) {
      toast(`Archive failed: ${res.detail}`, "error");
      return;
    }
    toast(`Archived ${res.data.archived} backlink${res.data.archived === 1 ? "" : "s"}`, "success");
    setReloadTick((t) => t + 1); // reload (also clears the selection)
  }

  // Monitoring level badge (Protected / Tracked / Inventory), colored.
  function monitorBadge(b: BacklinkView) {
    const lvl = b.monitoring_level ?? "inventory_only";
    const c = LEVEL_BADGE[lvl] ?? LEVEL_BADGE.inventory_only;
    return (
      <span className="bl-tag" style={{ background: c.bg, color: c.fg, borderColor: c.bg }}>
        {monitoringLabel(lvl)}
      </span>
    );
  }

  // One classification value as a chip (or "-" when unset).
  function classChip(slug?: string | null) {
    return slug ? <span className="bl-tag">{classLabel(slug)}</span> : <span className="muted">-</span>;
  }

  // Attribute cell: the manual override if set, else the crawler-detected value
  // (shown muted + italic to mark it as auto). "-" when never observed.
  function attrCell(b: BacklinkView) {
    if (b.link_attribute) return <span className="bl-tag">{classLabel(b.link_attribute)}</span>;
    if (b.link_attribute_auto)
      return (
        <span
          className="bl-tag"
          style={{ opacity: 0.65, fontStyle: "italic" }}
          title="Auto-detected from the crawler (no manual override)"
        >
          {classLabel(b.link_attribute_auto)}
        </span>
      );
    return <span className="muted">-</span>;
  }

  async function openDetail(b: BacklinkView) {
    setSelected(b);
    setEvents(null);
    setImpact(null);
    setBlTags([]);
    setOccurrences([]);
    setTier2([]);
    setT2csv("");
    setT2msg(null);
    setNewTag("");
    if (orgId) {
      const [ev, tg, oc, t2] = await Promise.all([
        orgApi.backlinkEvents(orgId, b.id),
        orgApi.backlinkTags(orgId, b.id),
        orgApi.backlinkOccurrences(orgId, b.id),
        orgApi.backlinkTier2(orgId, b.id),
      ]);
      setEvents(ev.ok ? ev.data : []);
      if (tg.ok) setBlTags(tg.data);
      if (oc.ok) setOccurrences(oc.data);
      if (t2.ok) setTier2(t2.data);
      // Post-Link Impact, only when Search Console is configured for this install.
      if (me?.features?.gsc) {
        const im = await orgApi.backlinkImpact(orgId, b.id);
        setImpact(im.ok ? im.data : null);
      }
    }
  }

  // Manual opt-in for a link that isn't auto-tracked, then refresh the view.
  async function enableImpact() {
    if (!orgId || !selected || impactBusy) return;
    setImpactBusy(true);
    const res = await orgApi.enableBacklinkImpact(orgId, selected.id);
    if (res.ok) {
      const im = await orgApi.backlinkImpact(orgId, selected.id);
      if (im.ok) setImpact(im.data);
      toast("Now tracking post-link impact", "success");
    } else {
      toast(res.detail || "Could not enable tracking", "error");
    }
    setImpactBusy(false);
  }

  async function importTier2() {
    if (!orgId || !selected) return;
    const rows = t2csv
      .split(/\r?\n/)
      .map((l) => l.trim())
      .filter(Boolean)
      .map((l) => {
        const [source_url, anchor, rel] = l.split(",").map((s) => s?.trim());
        return { source_url, anchor, rel };
      })
      .filter((r) => r.source_url);
    if (rows.length === 0) {
      setT2msg("Paste rows: source_url,anchor,rel");
      return;
    }
    const res = await orgApi.importTier2(orgId, selected.id, rows);
    if (!res.ok) {
      setT2msg(res.detail || "Import failed.");
      return;
    }
    setT2msg(`Imported ${res.data.imported} (${res.data.new} new).`);
    setT2csv("");
    const t2 = await orgApi.backlinkTier2(orgId, selected.id);
    if (t2.ok) setTier2(t2.data);
  }

  async function refreshBlTags(id: string) {
    if (!orgId) return;
    const tg = await orgApi.backlinkTags(orgId, id);
    if (tg.ok) setBlTags(tg.data);
  }

  async function attachExisting(tagId: string) {
    if (!orgId || !selected || !tagId) return;
    const res = await orgApi.attachTag(orgId, selected.id, tagId);
    if (res.ok) await refreshBlTags(selected.id);
  }

  async function removeTag(tagId: string) {
    if (!orgId || !selected) return;
    const res = await orgApi.detachTag(orgId, selected.id, tagId);
    if (res.ok) await refreshBlTags(selected.id);
  }

  async function createAndAttach() {
    if (!orgId || !selected) return;
    const name = newTag.trim();
    if (!name) return;
    const created = await orgApi.createTag(orgId, name);
    if (created.ok) {
      await orgApi.attachTag(orgId, selected.id, created.data.id);
      setNewTag("");
      const [tags, blt] = await Promise.all([orgApi.tags(orgId), orgApi.backlinkTags(orgId, selected.id)]);
      if (tags.ok) setOrgTags(tags.data);
      if (blt.ok) setBlTags(blt.data);
    }
  }

  async function archiveSelected() {
    if (!orgId || !selected || !projectId) return;
    const res = await orgApi.archiveBacklink(orgId, selected.id);
    if (res.ok) {
      setSelected(null);
      const list = await orgApi.backlinks(orgId, projectId);
      if (list.ok) setBacklinks(list.data);
    }
  }

  if (loading)
    return (
      <div className="dash" aria-busy="true" aria-label="Loading backlinks">
        <header className="page-head"><h1>Backlinks</h1></header>
        <div className="tiles">
          {Array.from({ length: 4 }).map((_, i) => (
            <div className="tile" key={i}><div className="sk-row" style={{ height: 52 }} /></div>
          ))}
        </div>
        <div className="skeleton-rows" style={{ marginTop: 18 }}>
          {Array.from({ length: 6 }).map((_, i) => (
            <div className="sk-row" key={i} />
          ))}
        </div>
      </div>
    );
  if (!me) return null;

  // The client these backlinks belong to: the explicit client filter, else the
  // selected project's client.
  const selectedProject = projects.find((p) => p.id === projectId) ?? null;
  const activeClientId = clientFilter || selectedProject?.client_id || "";
  const selectedClient = activeClientId
    ? clients.find((c) => c.id === activeClientId) ?? null
    : null;

  // Referring-domain filter (client-side, over the loaded list). Options are the
  // distinct source domains currently loaded; picking one narrows the table.
  const referringDomains = Array.from(
    new Set(backlinks.map((b) => b.source_domain).filter(Boolean)),
  ).sort();
  const visibleBacklinks = domainFilter
    ? backlinks.filter((b) => b.source_domain === domainFilter)
    : backlinks;

  async function createWorkspace(e: FormEvent) {
    e.preventDefault();
    const name = wsName.trim();
    if (name.length < 2) {
      setWsErr("Enter a workspace name (at least 2 characters).");
      return;
    }
    setWsErr(null);
    setWsBusy(true);
    const res = await api.createOrganization(name);
    if (!res.ok) {
      setWsBusy(false);
      setWsErr(res.detail || "Could not create the workspace.");
      return;
    }
    // Reload the session so the new org (and its id) propagate.
    const meRes = await api.me();
    setWsBusy(false);
    if (meRes.ok) {
      setMe(meRes.data);
      setOrgId(selectedOrgId(meRes.data.organizations));
      setWsName("");
    }
  }

  function openAdd() {
    setAddErr(null);
    setAddForm((f) => ({ ...f, project_id: projectId ?? projects[0]?.id ?? "" }));
    setShowAdd(true);
  }

  async function submitBacklink(e: FormEvent) {
    e.preventDefault();
    if (!orgId) return;
    const f = addForm;
    if (!f.project_id) {
      setAddErr("Pick a project for this backlink.");
      return;
    }
    const src = f.source_url.trim();
    const tgt = f.target_url.trim();
    if (!/^https?:\/\//.test(src) || !/^https?:\/\//.test(tgt)) {
      setAddErr("Source and target must start with http:// or https://.");
      return;
    }
    setAddErr(null);
    setAddBusy(true);
    const res = await orgApi.createBacklink(orgId, {
      project_id: f.project_id,
      source_url: src,
      target_url: tgt,
      expected_rel_type: f.expected_rel_type,
      monitor: monitorNew,
      ...(f.expected_anchor_text.trim() ? { expected_anchor_text: f.expected_anchor_text.trim() } : {}),
      ...(f.cost.trim() ? { cost: Number(f.cost), currency: f.currency.trim() || "USD" } : {}),
    });
    setAddBusy(false);
    if (!res.ok) {
      setAddErr(res.detail || "Could not add the backlink.");
      return;
    }
    toast(
      monitorNew ? "Backlink added, first check runs within ~10 minutes" : "Backlink added (not monitored)",
      "success",
    );
    setShowAdd(false);
    setAddForm({ project_id: f.project_id, source_url: "", target_url: "", expected_rel_type: "dofollow", expected_anchor_text: "", cost: "", currency: "" });
    // Show the project we just added to, and refresh the numbers + list.
    setClientFilter("");
    setProjectId(f.project_id);
    const [dash, list] = await Promise.all([
      orgApi.dashboard(orgId),
      orgApi.searchBacklinks(orgId, { projectId: f.project_id }),
    ]);
    if (dash.ok) setSummary(dash.data);
    if (list.ok) setBacklinks(list.data);
  }

  // Bulk add: one backlink per line as "source_url, target_url" (comma or tab).
  async function submitBulk() {
    if (!orgId) return;
    if (!addForm.project_id) {
      setAddErr("Pick a project first.");
      return;
    }
    const rows = bulkText
      .split("\n")
      .map((l) => l.trim())
      .filter(Boolean)
      .map((l) => {
        const [source_url, target_url] = l.split(/[,\t]/).map((s) => s?.trim());
        return { source_url, target_url };
      })
      .filter(
        (r) => /^https?:\/\//.test(r.source_url ?? "") && /^https?:\/\//.test(r.target_url ?? ""),
      ) as { source_url: string; target_url: string }[];
    if (rows.length === 0) {
      setAddErr("Add at least one line as: source URL, target URL (both http/https).");
      return;
    }
    setAddErr(null);
    setBulkAddBusy(true);
    const res = await orgApi.bulkCreateBacklinks(orgId, {
      project_id: addForm.project_id,
      backlinks: rows,
      monitor: monitorNew,
    });
    setBulkAddBusy(false);
    if (!res.ok) {
      setAddErr(res.detail || "Could not add the backlinks.");
      return;
    }
    const { created, duplicates } = res.data;
    toast(
      `Added ${created} backlink${created === 1 ? "" : "s"}` +
        (duplicates ? `, ${duplicates} already existed` : "") +
        (monitorNew ? ", first checks run within ~10 minutes" : ""),
      "success",
    );
    setBulkText("");
    setShowAdd(false);
    setClientFilter("");
    setProjectId(addForm.project_id);
    const [dash, list] = await Promise.all([
      orgApi.dashboard(orgId),
      orgApi.searchBacklinks(orgId, { projectId: addForm.project_id }),
    ]);
    if (dash.ok) setSummary(dash.data);
    if (list.ok) setBacklinks(list.data);
  }

  // Segments for the status-breakdown bar (only statuses that actually occur).
  const distSegments = STATUSES.map((st) => ({ st, n: summary?.counts[st] ?? 0 })).filter((s) => s.n > 0);
  const distTotal = distSegments.reduce((acc, s) => acc + s.n, 0);

  // Follow ratio (do/nofollow) illustration.
  const relSegments = [
    { label: "Dofollow", n: summary?.rel?.[0] ?? 0, color: "#36d399" },
    { label: "Nofollow", n: summary?.rel?.[1] ?? 0, color: "#fbbf24" },
  ].filter((s) => s.n > 0);
  const relTotal = relSegments.reduce((acc, s) => acc + s.n, 0);

  // Link-type illustration (auto-tagger categories).
  const TYPE_COLORS: Record<string, string> = {
    comment: "#22d3ee",
    pbn: "#fb7185",
    "forum-profile": "#a78bfa",
    "self-built": "#fbbf24",
    earned: "#36d399",
  };
  const typeSegments = ["earned", "self-built", "forum-profile", "pbn", "comment"]
    .map((k) => ({ label: k, n: summary?.link_types?.[k] ?? 0, color: TYPE_COLORS[k] }))
    .filter((s) => s.n > 0);
  const typeTotal = typeSegments.reduce((acc, s) => acc + s.n, 0);

  // Helpers for the command-center top section.
  const num = (n: number) => (n ?? 0).toLocaleString();
  const go = (route: string) => router.push(route);
  const scrollToList = () =>
    document.getElementById("bl-list")?.scrollIntoView({ behavior: "smooth", block: "start" });

  // Active-filter chips: a legible, reversible summary of the table drill-down.
  const labelOf = (pairs: readonly (readonly [string, string])[], v: string) =>
    pairs.find(([val]) => val === v)?.[1] ?? v;
  const activeFilters: { key: string; label: string; clear: () => void }[] = [];
  if (statusFilter)
    activeFilters.push({ key: "status", label: `Status: ${statusLabel(statusFilter)}`, clear: () => setStatusFilter("") });
  if (monitorFilter)
    activeFilters.push({ key: "monitor", label: `Monitoring: ${({ protected: "Protected", tracked: "Tracked", inventory_only: "Inventory" } as Record<string, string>)[monitorFilter] ?? monitorFilter}`, clear: () => setMonitorFilter("") });
  if (searchQ.trim())
    activeFilters.push({ key: "q", label: `“${searchQ.trim()}”`, clear: () => setSearchQ("") });
  if (clientFilter)
    activeFilters.push({ key: "client", label: `Client: ${clients.find((c) => c.id === clientFilter)?.name ?? "-"}`, clear: () => setClientFilter("") });
  if (tagFilter)
    activeFilters.push({ key: "tag", label: `Tag: ${orgTags.find((t) => t.id === tagFilter)?.name ?? "-"}`, clear: () => setTagFilter("") });
  if (lowValue)
    activeFilters.push({ key: "lowvalue", label: "Likely low-value", clear: () => setLowValue(false) });
  if (domainFilter)
    activeFilters.push({ key: "domain", label: `Domain: ${domainFilter}`, clear: () => setDomainFilter("") });
  if (acqFilter)
    activeFilters.push({ key: "acq", label: `Acquisition: ${labelOf(ACQUISITION, acqFilter)}`, clear: () => setAcqFilter("") });
  if (attrFilter)
    activeFilters.push({ key: "attr", label: `Attribute: ${labelOf(ATTRIBUTE, attrFilter)}`, clear: () => setAttrFilter("") });
  if (placeFilter)
    activeFilters.push({ key: "place", label: `Placement: ${labelOf(PLACEMENT, placeFilter)}`, clear: () => setPlaceFilter("") });
  if (fmtFilter)
    activeFilters.push({ key: "fmt", label: `Format: ${labelOf(FORMAT, fmtFilter)}`, clear: () => setFmtFilter("") });
  if (periodDays)
    activeFilters.push({ key: "period", label: `Last ${periodDays} days`, clear: () => setPeriodDays(0) });
  const clearAllFilters = () => {
    setStatusFilter("");
    setMonitorFilter("");
    setLowValue(false);
    setSearchQ("");
    setClientFilter("");
    setTagFilter("");
    setDomainFilter("");
    setAcqFilter("");
    setAttrFilter("");
    setPlaceFilter("");
    setFmtFilter("");
    setPeriodDays(0);
  };

  return (
    <main className="dash">
      <header className="page-head">
        <h1>Backlinks</h1>
      </header>

      {me.organizations.length === 0 ? (
        <section className="panel create-ws">
          <div className="empty-icon"><Icon name="rocket" size={26} /></div>
          <h2>Create your workspace</h2>
          <p className="muted">A workspace holds your clients, projects and backlinks. Name it after your agency or your own site.</p>
          <form onSubmit={createWorkspace} className="ws-form">
            <input
              type="text"
              placeholder="e.g. Acme SEO"
              value={wsName}
              onChange={(e) => setWsName(e.target.value)}
              disabled={wsBusy}
              autoFocus
            />
            <button type="submit" className="btn primary" disabled={wsBusy}>
              {wsBusy ? "Creating…" : "Create workspace"}
            </button>
          </form>
          {wsErr && <p className="error">{wsErr}</p>}
        </section>
      ) : (
        <>
          {summary && (
            <section className="bl-agg">
              <div className="bl-agg-head">
                <h2>Overview</h2>
                <span
                  className="bl-scope"
                  title="These aggregates cover every project in this workspace. The backlinks table below is filtered to the selected project."
                >
                  Across all projects
                </span>
              </div>

              <div className="bl-kpis">
                <BlKpi
                  label="Active backlinks"
                  icon="link"
                  accent="var(--live)"
                  value={num(summary?.counts?.live ?? 0)}
                  hint="live now"
                  onClick={() => {
                    setStatusFilter("live");
                    scrollToList();
                  }}
                />
                <BlKpi
                  label="New"
                  icon="signal"
                  accent="var(--live)"
                  value={overview ? `+${num(overview.backlink_growth.new_count)}` : "-"}
                  hint="recently added"
                  onClick={() => {
                    setStatusFilter("");
                    scrollToList();
                  }}
                />
                <BlKpi
                  label="Lost"
                  icon="warn"
                  accent="var(--lost)"
                  value={num(summary?.counts?.lost ?? 0)}
                  hint="currently lost"
                  onClick={() => {
                    setStatusFilter("lost");
                    scrollToList();
                  }}
                />
                <BlKpi
                  label="Link health"
                  icon="check"
                  value={
                    overview && overview.summary.link_health.percentage != null
                      ? `${Math.round(overview.summary.link_health.percentage)}%`
                      : "-"
                  }
                  delta={overview?.summary.link_health.change ?? null}
                  hint={
                    overview
                      ? `${num(overview.summary.link_health.healthy)}/${num(overview.summary.link_health.eligible)} healthy`
                      : "monitored links"
                  }
                  onClick={() => {
                    setMonitorFilter("protected");
                    scrollToList();
                  }}
                />
              </div>

              {overview && (
              <div className="bl-two">
                <section className="panel">
                  <div className="panel-head">
                    <h2>Backlink evolution</h2>
                  </div>
                  <GrowthChart series={overview.backlink_growth.series} />
                  <div className="bl-evo-foot">
                    <span>
                      <b style={{ color: "var(--live)" }}>+{num(overview.backlink_growth.new_count)}</b>New
                    </span>
                    <span>
                      <b style={{ color: "var(--lost)" }}>-{num(overview.backlink_growth.lost_count)}</b>Lost
                    </span>
                    <span>
                      <b
                        style={{
                          color: overview.backlink_growth.net_count >= 0 ? "var(--live)" : "var(--lost)",
                        }}
                      >
                        {overview.backlink_growth.net_count >= 0 ? "+" : ""}
                        {num(overview.backlink_growth.net_count)}
                      </b>
                      Net
                    </span>
                  </div>
                </section>

                <section className="panel">
                  <div className="panel-head">
                    <h2>Insights</h2>
                    {overview.attention.length > 0 && (
                      <span className="badge">{overview.attention.length}</span>
                    )}
                  </div>
                  {overview.next_best_action && (
                    <div className="bl-nba">
                      <div>
                        <div className="attn-title">{overview.next_best_action.title}</div>
                        {overview.next_best_action.subtitle && (
                          <div className="attn-sub">{overview.next_best_action.subtitle}</div>
                        )}
                      </div>
                      <button
                        className="btn sm primary"
                        onClick={() => go(overview.next_best_action!.route)}
                      >
                        {overview.next_best_action.action}
                      </button>
                    </div>
                  )}
                  {overview.attention.length === 0 ? (
                    <div className="bl-insights-empty">
                      <Icon name="check" size={22} />
                      <span className="bl-insights-ok">All clear</span>
                      <span>No backlinks need your attention right now.</span>
                    </div>
                  ) : (
                    <div className="attention-list">
                      {overview.attention.slice(0, 5).map((a: AttentionItemView, i: number) => (
                        <div className="attn-item" key={i}>
                          <span className={`attn-dot ${a.priority}`} />
                          <div className="attn-main">
                            <div className="attn-title">{a.title}</div>
                            {a.subtitle && <div className="attn-sub">{a.subtitle}</div>}
                          </div>
                          {a.value_at_risk && (
                            <span className="attn-value">
                              {a.value_at_risk.amount} {a.value_at_risk.currency}
                            </span>
                          )}
                          <button className="btn sm" onClick={() => go(a.route)}>
                            {a.action}
                          </button>
                        </div>
                      ))}
                    </div>
                  )}
                </section>
              </div>
              )}
            </section>
          )}

          {summary && (distTotal > 0 || relTotal > 0 || typeTotal > 0) && (
            <div className="dist-row triple">
              {relTotal > 0 && (
                <MiniDonut
                  title="Follow ratio"
                  head={`${relTotal} link${relTotal > 1 ? "s" : ""}`}
                  segments={relSegments.map((s) => ({ key: s.label, label: s.label, n: s.n, color: s.color }))}
                  center={{
                    value: `${Math.round(((relSegments.find((s) => s.label === "Dofollow")?.n ?? 0) / relTotal) * 100)}%`,
                    label: "dofollow",
                  }}
                />
              )}
              {distTotal > 0 && (
                <MiniDonut
                  title="Status breakdown"
                  head={`${distTotal} link${distTotal > 1 ? "s" : ""}`}
                  segments={distSegments.map((s) => ({
                    key: s.st,
                    label: statusLabel(s.st),
                    n: s.n,
                    color: statusColor(s.st),
                  }))}
                  center={{
                    value: `${Math.round(((distSegments.find((s) => s.st === "live")?.n ?? 0) / distTotal) * 100)}%`,
                    label: "live",
                  }}
                />
              )}
              {typeTotal > 0 && (
                <MiniDonut
                  title="Link type"
                  head={
                    <>
                      {typeTotal} tagged ·{" "}
                      <a href="#" onClick={(e) => { e.preventDefault(); void runAutoTag(); }}>re-tag</a>
                    </>
                  }
                  segments={typeSegments.map((s) => ({ key: s.label, label: s.label, n: s.n, color: s.color }))}
                  center={{
                    value: `${Math.round(((typeSegments.find((s) => s.label === "earned")?.n ?? 0) / typeTotal) * 100)}%`,
                    label: "earned",
                  }}
                />
              )}
            </div>
          )}

          {anchors && <AnchorDistribution data={anchors} />}

          <section className="panel" id="bl-list">
            <div className="panel-head">
              <h2>
                Backlinks
                {selectedClient && <span className="client-badge">{selectedClient.name}</span>}
              </h2>
              <div className="panel-actions">
                {clients.length > 0 && (
                  <select value={clientFilter} onChange={(e) => setClientFilter(e.target.value)} title="Filter by client">
                    <option value="">All clients</option>
                    {clients.map((c) => (
                      <option key={c.id} value={c.id}>{c.name}</option>
                    ))}
                  </select>
                )}
                {projects.length > 0 && (
                  <select
                    value={projectId ?? ""}
                    onChange={(e) => setProjectId(e.target.value)}
                    disabled={!!clientFilter}
                    title={clientFilter ? "Clear the client filter to pick a project" : "Project"}
                  >
                    {projects.map((p) => (
                      <option key={p.id} value={p.id}>{p.name}, {p.domain}</option>
                    ))}
                  </select>
                )}
                <input
                  className="bl-search"
                  placeholder="Search domain/anchor"
                  aria-label="Search backlinks by domain or anchor"
                  value={searchQ}
                  onChange={(e) => setSearchQ(e.target.value)}
                />
                <select
                  aria-label="Filter by status"
                  value={statusFilter}
                  onChange={(e) => setStatusFilter(e.target.value)}
                >
                  <option value="">All statuses</option>
                  {STATUSES.map((st) => (
                    <option key={st} value={st}>{statusLabel(st)}</option>
                  ))}
                </select>
                <select
                  value={periodDays}
                  onChange={(e) => setPeriodDays(Number(e.target.value))}
                  title="Filter by acquisition period (first seen)"
                >
                  <option value={0}>All time</option>
                  <option value={7}>Last 7 days</option>
                  <option value={30}>Last 30 days</option>
                  <option value={90}>Last 90 days</option>
                </select>
                <select
                  value={domainFilter}
                  onChange={(e) => setDomainFilter(e.target.value)}
                  title="Filter by referring domain"
                >
                  <option value="">All domains</option>
                  {referringDomains.map((d) => (
                    <option key={d} value={d}>{d}</option>
                  ))}
                </select>
                {orgTags.length > 0 && (
                  <select
                    value={tagFilter}
                    onChange={(e) => setTagFilter(e.target.value)}
                    title="Filter by tag"
                  >
                    <option value="">All tags</option>
                    {orgTags.map((t) => (
                      <option key={t.id} value={t.id}>{t.name}</option>
                    ))}
                  </select>
                )}
                <select value={acqFilter} onChange={(e) => setAcqFilter(e.target.value)} title="Filter by acquisition">
                  <option value="">All acquisition</option>
                  <option value="none">(Unclassified)</option>
                  {ACQUISITION.map(([v, l]) => <option key={v} value={v}>{l}</option>)}
                </select>
                <select value={attrFilter} onChange={(e) => setAttrFilter(e.target.value)} title="Filter by link attribute">
                  <option value="">All attributes</option>
                  <option value="none">(Unclassified)</option>
                  {ATTRIBUTE.map(([v, l]) => <option key={v} value={v}>{l}</option>)}
                </select>
                <select value={placeFilter} onChange={(e) => setPlaceFilter(e.target.value)} title="Filter by placement">
                  <option value="">All placements</option>
                  <option value="none">(Unclassified)</option>
                  {PLACEMENT.map(([v, l]) => <option key={v} value={v}>{l}</option>)}
                </select>
                <select value={fmtFilter} onChange={(e) => setFmtFilter(e.target.value)} title="Filter by format">
                  <option value="">All formats</option>
                  <option value="none">(Unclassified)</option>
                  {FORMAT.map(([v, l]) => <option key={v} value={v}>{l}</option>)}
                </select>
                {projects.length > 0 && (
                  <button className="btn sm primary" onClick={openAdd}>+ Add backlink</button>
                )}
                <a className="btn sm" href="/import">Import</a>
                {orgId && projectId && (
                  <button
                    className="btn sm"
                    disabled={autoTagBusy}
                    title="Classify this project's backlinks by footprint (comment / PBN / forum / self-built / earned)"
                    onClick={() => void runAutoTag()}
                  >
                    {autoTagBusy ? "Tagging…" : <><Icon name="tag" size={14} /> Auto-tag</>}
                  </button>
                )}
                {orgId && (
                  <a className="btn sm" href={backlinksExportUrl(orgId, projectId ?? undefined, clientFilter || undefined)}>
                    Export CSV
                  </a>
                )}
              </div>
            </div>
            {warranties.length > 0 && (
              <div className="warranty-note">
                <Icon name="alert" size={15} /> {warranties.length} backlink{warranties.length > 1 ? "s" : ""} with a warranty
                expiring soon or lapsed:{" "}
                {warranties
                  .slice(0, 5)
                  .map((w) => `${w.source_domain} (${w.warranty_status ?? "?"})`)
                  .join(", ")}
                {warranties.length > 5 ? "…" : ""}
              </div>
            )}
            {/* Monitoring view tabs */}
            <div style={{ display: "flex", gap: 6, margin: "10px 0 6px", flexWrap: "wrap" }}>
              {([["", "All"], ["protected", "Protected"], ["tracked", "Tracked"], ["inventory_only", "Inventory"]] as const).map(
                ([v, l]) => (
                  <button
                    key={v || "all"}
                    className={`btn sm${monitorFilter === v ? " primary" : ""}`}
                    onClick={() => setMonitorFilter(v)}
                  >
                    {l}
                  </button>
                ),
              )}
              <span style={{ width: 1, alignSelf: "stretch", background: "var(--border)", margin: "2px 4px" }} aria-hidden />
              <button
                className={`btn sm${lowValue ? " primary" : ""}`}
                onClick={() => setLowValue((v) => !v)}
                title="Links the footprint detector flagged as spam/self-built (comment, PBN, forum profile, self-built). Run “Detect footprints” first."
              >
                Likely low-value
              </button>
            </div>
            {/* Active-filter chips: see what's applied, remove one or clear all */}
            {activeFilters.length > 0 && (
              <div className="bl-chips">
                <span className="bl-chips-count">
                  {backlinks.length}
                  {hasNext ? "+" : ""} shown
                </span>
                {activeFilters.map((f) => (
                  <button key={f.key} className="bl-chip" onClick={f.clear} title="Remove this filter">
                    {f.label} <span aria-hidden>×</span>
                  </button>
                ))}
                <button className="bl-chip-clear" onClick={clearAllFilters}>
                  Clear all
                </button>
              </div>
            )}
            {/* Bulk action bar (shown when rows are selected) */}
            {selectedIds.size > 0 && (
              <div
                style={{
                  display: "flex",
                  gap: 8,
                  alignItems: "center",
                  margin: "0 0 10px",
                  padding: "8px 10px",
                  border: "1px solid var(--border)",
                  borderRadius: 8,
                  flexWrap: "wrap",
                }}
              >
                <span className="sm"><b>{selectedIds.size}</b> selected</span>
                <button className="btn sm primary" disabled={bulkBusy} onClick={() => void bulkMonitor({ monitoring_enabled: true, monitoring_level: "protected" })}>Protect</button>
                <button className="btn sm" disabled={bulkBusy} onClick={() => void bulkMonitor({ monitoring_enabled: true, monitoring_level: "tracked" })}>Track</button>
                <button className="btn sm" disabled={bulkBusy} onClick={() => void bulkMonitor({ monitoring_enabled: false, monitoring_level: "inventory_only" })}>Move to Inventory</button>
                <select
                  disabled={bulkBusy}
                  defaultValue=""
                  onChange={(e) => { const v = e.target.value; e.currentTarget.value = ""; if (v) void bulkMonitor({ check_frequency: v }); }}
                  title="Set check frequency for the selection"
                >
                  <option value="">Set frequency…</option>
                  {CHECK_FREQUENCY.map(([v, l]) => <option key={v} value={v}>{l}</option>)}
                </select>
                <button className="btn sm danger" disabled={bulkBusy} onClick={() => void bulkArchive()} title="Archive the selected links (reversible)">Archive</button>
                <button className="btn sm" disabled={bulkBusy} onClick={() => setSelectedIds(new Set())}>Clear</button>
              </div>
            )}
            {backlinks.length === 0 ? (
              <div className="empty">
                <div className="empty-icon"><Icon name="link" size={30} /></div>
                <h3>{projectId || clientFilter ? "No backlinks here yet" : "Pick a project to view its backlinks"}</h3>
                <p>
                  {projects.length === 0
                    ? "Create a project first, then import the backlinks you want to monitor."
                    : "Import a CSV of backlinks and LinkPulz will track their status, anchors, rel and warranties."}
                </p>
                <div className="empty-cta">
                  {projects.length === 0 ? (
                    <>
                      <a className="btn primary" href="/projects">Create a project</a>
                      <a className="btn" href="/projects">Manage projects</a>
                    </>
                  ) : (
                    <>
                      <button className="btn primary" onClick={openAdd}>+ Add backlink</button>
                      <a className="btn" href="/import">Import CSV</a>
                    </>
                  )}
                </div>
              </div>
            ) : (
              <div className="bl-scroll">
              <table className="bl-table">
                <thead>
                  <tr>
                    <th style={{ width: 28 }}>
                      <input
                        type="checkbox"
                        aria-label="Select all on page"
                        checked={visibleBacklinks.length > 0 && selectedIds.size === visibleBacklinks.length}
                        onChange={() =>
                          setSelectedIds((s) =>
                            s.size === visibleBacklinks.length && visibleBacklinks.length > 0
                              ? new Set()
                              : new Set(visibleBacklinks.map((b) => b.id)),
                          )
                        }
                      />
                    </th>
                    <th>Status</th>
                    <th>Monitoring</th>
                    <th>Source</th>
                    <th>Target</th>
                    <th>Anchor</th>
                    <th>Acquisition</th>
                    <th>Attribute</th>
                    <th>Placement</th>
                    <th>Format</th>
                    <th>Rel</th>
                    <th>HTTP</th>
                    <th>Checked</th>
                    <th>Next check</th>
                  </tr>
                </thead>
                <tbody>
                  {visibleBacklinks.length === 0 && (
                    <tr>
                      <td colSpan={14} className="muted" style={{ padding: 16 }}>
                        No backlinks from <b className="mono">{domainFilter}</b> in this view.
                      </td>
                    </tr>
                  )}
                  {visibleBacklinks.map((b) => (
                    <tr
                      key={b.id}
                      className="row-click"
                      role="button"
                      tabIndex={0}
                      aria-label={`Open backlink from ${b.source_domain}`}
                      onClick={() => void openDetail(b)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          e.preventDefault();
                          void openDetail(b);
                        }
                      }}
                    >
                      <td onClick={(e) => e.stopPropagation()}>
                        <input
                          type="checkbox"
                          aria-label="Select backlink"
                          checked={selectedIds.has(b.id)}
                          onChange={() => toggleSelect(b.id)}
                        />
                      </td>
                      <td><span className="badge" style={{ "--st": statusColor(b.status) } as CSSProperties}><span className="dot" />{statusLabel(b.status)}</span></td>
                      <td>{monitorBadge(b)}</td>
                      <td className="cell-domain" title={b.source_url}>
                        {b.source_domain}
                        {b.source_noindex && (
                          <span
                            className="noindex-badge"
                            title="Source page is noindexed — Google won't index it, so this link passes little or no value"
                          >
                            noindex
                          </span>
                        )}
                        {b.source_ip && (
                          <span className="src-ip" title="Resolved IP of the source host at the last check">
                            {b.source_ip}
                          </span>
                        )}
                      </td>
                      <td className="cell-domain target" title={b.target_url}>{b.target_domain}</td>
                      <td className="cell-anchor" title={b.detected_anchor_text ?? b.expected_anchor_text ?? ""}>{b.detected_anchor_text ?? b.expected_anchor_text ?? "-"}</td>
                      <td>{classChip(b.acquisition)}</td>
                      <td>{attrCell(b)}</td>
                      <td>{classChip(b.placement)}</td>
                      <td>{classChip(b.link_format)}</td>
                      <td><RelChips b={b} /></td>
                      <td className="mono">{httpPair(b)}</td>
                      <td className="mono">{fmtShort(b.last_checked_at)}</td>
                      <td className="mono">{b.next_check_at ? fmtShort(b.next_check_at) : "-"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
              </div>
            )}
            {(backlinks.length > 0 || page > 0) && (
              <div className="pager">
                <button
                  className="btn sm"
                  disabled={page === 0}
                  onClick={() => setPage((p) => Math.max(0, p - 1))}
                >
                  ← Prev
                </button>
                <span className="muted">
                  Page {page + 1}
                  {domainFilter ? ` · ${visibleBacklinks.length} shown` : ""}
                </span>
                <button className="btn sm" disabled={!hasNext} onClick={() => setPage((p) => p + 1)}>
                  Next →
                </button>
                <select
                  value={pageSize}
                  onChange={(e) => setPageSize(Number(e.target.value))}
                  title="Rows per page"
                  style={{ marginLeft: 8 }}
                >
                  {[10, 50, 100].map((n) => (
                    <option key={n} value={n}>{n} / page</option>
                  ))}
                </select>
              </div>
            )}
          </section>
        </>
      )}
      {showAdd && (
        <div className="drawer-backdrop" onClick={() => setShowAdd(false)}>
          <aside
            className="drawer"
            role="dialog"
            aria-modal="true"
            aria-label="Add backlinks"
            tabIndex={-1}
            ref={addDrawerRef}
            onClick={(e) => e.stopPropagation()}
          >
            <div className="panel-head">
              <h2>Add backlinks</h2>
              <button className="btn" onClick={() => setShowAdd(false)}>Close</button>
            </div>
            <div className="add-form">
              <label>
                <span>Project</span>
                <select
                  value={addForm.project_id}
                  onChange={(e) => setAddForm((f) => ({ ...f, project_id: e.target.value }))}
                >
                  {projects.map((p) => (
                    <option key={p.id} value={p.id}>{p.name}, {p.domain}</option>
                  ))}
                </select>
              </label>

              <label
                style={{ display: "flex", gap: 10, alignItems: "flex-start", margin: "4px 0 10px", fontSize: 13 }}
              >
                <input
                  type="checkbox"
                  checked={monitorNew}
                  onChange={(e) => setMonitorNew(e.target.checked)}
                  style={{ marginTop: 2 }}
                />
                <span>
                  <b>Monitor these links</b>, re-check them on a schedule and alert me the moment
                  one is lost, redirected, nofollowed or broken. <span className="muted">(Uncheck to
                  just store them without monitoring.)</span>
                </span>
              </label>

              <label>
                <span>
                  Paste your backlinks, one per line: <code>source URL, target URL</code>
                </span>
                <textarea
                  rows={7}
                  placeholder={
                    "https://blog.example.com/post, https://yourdomain.com/page\nhttps://news.example.org/article, https://yourdomain.com/"
                  }
                  value={bulkText}
                  onChange={(e) => setBulkText(e.target.value)}
                  style={{
                    width: "100%",
                    minHeight: 140,
                    fontFamily: "ui-monospace, monospace",
                    fontSize: 12.5,
                    padding: "8px 10px",
                    background: "var(--bg)",
                    color: "var(--text)",
                    border: "1px solid var(--border)",
                    borderRadius: 8,
                  }}
                />
              </label>
              {addErr && <p className="error">{addErr}</p>}
              <button className="btn primary" disabled={bulkAddBusy} onClick={() => void submitBulk()}>
                {bulkAddBusy ? "Adding…" : "Add backlinks"}
              </button>
              <p className="muted" style={{ fontSize: 12, marginTop: 8 }}>
                Have a spreadsheet? <a href="/import">Import a CSV →</a>
              </p>

              <button
                type="button"
                className="btn sm ghost"
                style={{ marginTop: 14 }}
                onClick={() => setShowSingle((v) => !v)}
              >
                {showSingle ? "Hide single-link form" : "Add one link with details (anchor, cost, rel)…"}
              </button>

              {showSingle && (
                <form
                  onSubmit={submitBacklink}
                  className="add-form"
                  style={{ marginTop: 12, paddingTop: 12, borderTop: "1px solid var(--border)" }}
                >
                  <label>
                    <span>Source URL (the page that links to you)</span>
                    <input
                      placeholder="https://blog.example.com/post"
                      value={addForm.source_url}
                      onChange={(e) => setAddForm((f) => ({ ...f, source_url: e.target.value }))}
                    />
                  </label>
                  <label>
                    <span>Target URL (your page)</span>
                    <input
                      placeholder="https://yourdomain.com/landing"
                      value={addForm.target_url}
                      onChange={(e) => setAddForm((f) => ({ ...f, target_url: e.target.value }))}
                    />
                  </label>
                  <div className="add-row">
                    <label>
                      <span>Rel</span>
                      <select
                        value={addForm.expected_rel_type}
                        onChange={(e) => setAddForm((f) => ({ ...f, expected_rel_type: e.target.value }))}
                      >
                        <option value="dofollow">dofollow</option>
                        <option value="nofollow">nofollow</option>
                        <option value="sponsored">sponsored</option>
                        <option value="ugc">ugc</option>
                      </select>
                    </label>
                    <label>
                      <span>Anchor text (optional)</span>
                      <input
                        placeholder="e.g. best SEO tools"
                        value={addForm.expected_anchor_text}
                        onChange={(e) => setAddForm((f) => ({ ...f, expected_anchor_text: e.target.value }))}
                      />
                    </label>
                  </div>
                  <div className="add-row">
                    <label>
                      <span>Cost (optional)</span>
                      <input
                        type="number"
                        min="0"
                        step="0.01"
                        placeholder="0.00"
                        value={addForm.cost}
                        onChange={(e) => setAddForm((f) => ({ ...f, cost: e.target.value }))}
                      />
                    </label>
                    <label>
                      <span>Currency</span>
                      <input
                        list="bl-currencies"
                        placeholder="EUR"
                        maxLength={3}
                        title="Pick one or type another 3-letter code (e.g. EUR, USD, GBP)"
                        value={addForm.currency}
                        onChange={(e) =>
                          setAddForm((f) => ({
                            ...f,
                            currency: e.target.value.toUpperCase().replace(/[^A-Z]/g, "").slice(0, 3),
                          }))
                        }
                      />
                      <datalist id="bl-currencies">
                        <option value="EUR">Euro</option>
                        <option value="USD">US Dollar</option>
                        <option value="GBP">British Pound</option>
                        <option value="CAD">Canadian Dollar</option>
                        <option value="AUD">Australian Dollar</option>
                        <option value="CHF">Swiss Franc</option>
                        <option value="JPY">Japanese Yen</option>
                      </datalist>
                    </label>
                  </div>
                  <button className="btn primary" type="submit" disabled={addBusy}>
                    {addBusy ? "Adding…" : "Add this backlink"}
                  </button>
                </form>
              )}
            </div>
          </aside>
        </div>
      )}
      {selected && (
        <div className="drawer-backdrop" onClick={() => setSelected(null)}>
          <aside
            className="drawer"
            role="dialog"
            aria-modal="true"
            aria-label="Backlink details"
            tabIndex={-1}
            ref={detailDrawerRef}
            onClick={(e) => e.stopPropagation()}
          >
            <div className="panel-head">
              <h2>Backlink history</h2>
              <div className="panel-actions">
                <button className="btn sm" onClick={() => void archiveSelected()}>Archive</button>
                <button className="btn" onClick={() => setSelected(null)}>Close</button>
              </div>
            </div>
            <p style={{ margin: "6px 0 10px" }}>
              <span className="badge" style={{ "--st": statusColor(selected.status) } as CSSProperties}><span className="dot" />{statusLabel(selected.status)}</span>
            </p>
            <p
              className="muted"
              style={{ margin: "0 0 12px", fontSize: ".86rem", display: "flex", gap: 6, alignItems: "flex-start" }}
            >
              <Icon name="signal" size={14} />
              <span>
                The values below (found anchor, rel, URLs, HTTP status, dates…) are{" "}
                <b>detected automatically</b> each time LinkPulz checks this backlink, they aren’t
                entered by hand.
                {selected.status === "pending" &&
                  " This link hasn’t been checked yet, so they show “-”; they fill in after the first check (within ~10 min for monitored links)."}{" "}
                To change the <b>expected</b> anchor, the cost/currency or the classification, use the
                sections further down.
              </span>
            </p>
            <dl className="detail-grid">
              <div><dt>Source</dt><dd className="mono br">{selected.source_url}</dd></div>
              <div><dt>Target</dt><dd className="mono br">{selected.target_url}</dd></div>
              <div><dt>Target final URL</dt><dd className="mono br">{selected.target_final_url ?? "-"}</dd></div>
              <div><dt>Link URL (found)</dt><dd className="mono br">{selected.detected_target_url ?? "-"}</dd></div>
              <div><dt>Anchor (found)</dt><dd>{selected.detected_anchor_text ?? "-"}</dd></div>
              <div><dt>Anchor (expected)</dt><dd>{selected.expected_anchor_text ?? "-"}</dd></div>
              <div><dt>Rel (found)</dt><dd className="mono">{selected.detected_rel?.join(" ") || "-"}</dd></div>
              <div><dt>dofollow / nofollow</dt><dd>{yn(selected.is_dofollow)} / {yn(selected.is_nofollow)}</dd></div>
              <div><dt>sponsored / ugc</dt><dd>{yn(selected.is_sponsored)} / {yn(selected.is_ugc)}</dd></div>
              <div><dt>Link type</dt><dd>{selected.link_type ?? "-"}</dd></div>
              <div><dt>Image alt</dt><dd>{selected.link_type === "image" ? (selected.image_alt || "(empty)") : "-"}</dd></div>
              <div><dt>HTTP source / target</dt><dd className="mono">{httpPair(selected)}</dd></div>
              <div><dt>First seen</dt><dd>{fmtLong(selected.first_checked_at)}</dd></div>
              <div><dt>Last seen</dt><dd>{fmtLong(selected.last_seen_at)}</dd></div>
              <div><dt>Last checked</dt><dd>{fmtLong(selected.last_checked_at)}</dd></div>
              <div><dt>Lost at</dt><dd>{fmtLong(selected.lost_at)}</dd></div>
              <div><dt>Cost</dt><dd>{selected.cost ? `${selected.cost} ${selected.currency ?? ""}` : "-"}</dd></div>
            </dl>
            <h3 className="drawer-sub">Classification</h3>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10, marginBottom: 14 }}>
              {([
                ["Acquisition", "acquisition", ACQUISITION, "Unclassified"],
                ["Attribute", "link_attribute", ATTRIBUTE, "Auto (detected)"],
                ["Placement", "placement", PLACEMENT, "Unclassified"],
                ["Format", "link_format", FORMAT, "Unclassified"],
              ] as const).map(([label, field, opts, blank]) => (
                <label key={field} style={{ display: "flex", flexDirection: "column", gap: 4, fontSize: 12 }}>
                  <span className="muted">
                    {label}
                    {field === "link_attribute" && selected.link_attribute == null && selected.link_attribute_auto
                      ? ` · auto: ${classLabel(selected.link_attribute_auto)}`
                      : ""}
                  </span>
                  <select
                    value={(selected[field] as string | null | undefined) ?? ""}
                    disabled={classBusy}
                    onChange={(e) => void setClass(field, e.target.value)}
                  >
                    <option value="">{blank}</option>
                    {opts.map(([v, l]) => <option key={v} value={v}>{l}</option>)}
                  </select>
                </label>
              ))}
            </div>
            <h3 className="drawer-sub">Monitoring</h3>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10, marginBottom: 6 }}>
              <label style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13 }}>
                <input
                  type="checkbox"
                  checked={!!selected.monitoring_enabled}
                  onChange={(e) =>
                    void patchBacklink({
                      monitoring_enabled: e.target.checked,
                      monitoring_level: e.target.checked
                        ? (selected.monitoring_level ?? "inventory_only") === "inventory_only"
                          ? "tracked"
                          : (selected.monitoring_level ?? "tracked")
                        : "inventory_only",
                    })
                  }
                />
                Monitoring active
              </label>
              <label className="auth-field" style={{ margin: 0, fontSize: 12 }}>
                <span className="muted">Level</span>
                <select
                  value={selected.monitoring_level ?? "inventory_only"}
                  onChange={(e) =>
                    void patchBacklink({
                      monitoring_level: e.target.value,
                      monitoring_enabled: e.target.value !== "inventory_only",
                    })
                  }
                >
                  {MONITORING_LEVEL.map(([v, l]) => <option key={v} value={v}>{l}</option>)}
                </select>
              </label>
              <label className="auth-field" style={{ margin: 0, fontSize: 12 }}>
                <span className="muted">Check frequency</span>
                <select
                  value={selected.check_frequency ?? "weekly"}
                  onChange={(e) => void patchBacklink({ check_frequency: e.target.value })}
                >
                  {CHECK_FREQUENCY.map(([v, l]) => <option key={v} value={v}>{l}</option>)}
                </select>
              </label>
              <div className="sm muted" style={{ alignSelf: "center" }}>
                {selected.monitoring_reason ? `Reason: ${monitoringLabel(selected.monitoring_reason)}` : ""}
                {selected.next_check_at ? ` · next: ${fmtShort(selected.next_check_at)}` : ""}
              </div>
            </div>
            <h3 className="drawer-sub">Economic</h3>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10, marginBottom: 14 }}>
              <label className="auth-field" style={{ margin: 0, fontSize: 12 }}>
                <span className="muted">Cost</span>
                <input
                  key={`cost-${selected.id}`}
                  ref={costRef}
                  type="number"
                  min="0"
                  step="0.01"
                  defaultValue={selected.cost ?? ""}
                  placeholder="e.g. 50"
                  onBlur={saveCostCurrency}
                />
              </label>
              <label className="auth-field" style={{ margin: 0, fontSize: 12 }}>
                <span className="muted">Currency</span>
                <input
                  key={`cur-${selected.id}`}
                  ref={currencyRef}
                  list="bl-currencies-edit"
                  maxLength={3}
                  defaultValue={selected.currency ?? ""}
                  placeholder="EUR"
                  title="3-letter code, e.g. EUR, USD, GBP"
                  onBlur={saveCostCurrency}
                />
                <datalist id="bl-currencies-edit">
                  <option value="EUR">Euro</option>
                  <option value="USD">US Dollar</option>
                  <option value="GBP">British Pound</option>
                  <option value="CAD">Canadian Dollar</option>
                  <option value="AUD">Australian Dollar</option>
                  <option value="CHF">Swiss Franc</option>
                  <option value="JPY">Japanese Yen</option>
                </datalist>
              </label>
              <label className="auth-field" style={{ margin: 0, fontSize: 12 }}>
                <span className="muted">Commercial model</span>
                <select
                  value={selected.commercial_model ?? ""}
                  onChange={(e) => void patchBacklink({ commercial_model: e.target.value || null })}
                >
                  <option value="">-</option>
                  {COMMERCIAL_MODEL.map(([v, l]) => <option key={v} value={v}>{l}</option>)}
                </select>
              </label>
              <label className="auth-field" style={{ margin: 0, fontSize: 12 }}>
                <span className="muted">Purchase date</span>
                <input
                  type="date"
                  value={selected.purchase_date ?? ""}
                  onChange={(e) => void patchBacklink({ purchase_date: e.target.value || null })}
                />
              </label>
              <label className="auth-field" style={{ margin: 0, fontSize: 12 }}>
                <span className="muted">Renewal date</span>
                <input
                  type="date"
                  value={selected.renewal_date ?? ""}
                  onChange={(e) => void patchBacklink({ renewal_date: e.target.value || null })}
                />
              </label>
              <label className="auth-field" style={{ margin: 0, fontSize: 12 }}>
                <span className="muted">Order reference</span>
                <input
                  key={selected.id}
                  defaultValue={selected.order_reference ?? ""}
                  placeholder="invoice / PO #"
                  onBlur={(e) => {
                    const v = e.target.value.trim();
                    if (v !== (selected.order_reference ?? "")) void patchBacklink({ order_reference: v || null });
                  }}
                />
              </label>
              <label style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13 }}>
                <input
                  type="checkbox"
                  checked={!!selected.replacement_eligible}
                  onChange={(e) => void patchBacklink({ replacement_eligible: e.target.checked })}
                />
                Replacement eligible
              </label>
            </div>
            {occurrences.length > 0 && (
              <>
                <h3 className="drawer-sub">Occurrences ({occurrences.length})</h3>
                <ul className="occ-list">
                  {occurrences.map((o) => (
                    <li key={o.id} className="occ">
                      <div className="occ-top">
                        <span className="occ-anchor">
                          {o.anchor_text ||
                            (o.link_type === "image"
                              ? `[image${o.image_alt ? ": " + o.image_alt : ""}]`
                              : "-")}
                        </span>
                        <span className="occ-place">{o.placement}</span>
                      </div>
                      <div className="rel-chips">
                        {o.link_type === "image" && <span className="rel-chip">image</span>}
                        {o.is_nofollow ? (
                          <span className="rel-chip rel-nofollow">nofollow</span>
                        ) : (
                          <span className="rel-chip rel-dofollow">dofollow</span>
                        )}
                        {o.is_sponsored && <span className="rel-chip rel-sponsored">sponsored</span>}
                        {o.is_ugc && <span className="rel-chip rel-ugc">ugc</span>}
                      </div>
                      {o.surrounding_text && <div className="occ-ctx muted">“{o.surrounding_text}”</div>}
                    </li>
                  ))}
                </ul>
              </>
            )}
            <h3 className="drawer-sub">Tier 2, reinforcing links ({tier2.length})</h3>
            {tier2.length === 0 ? (
              <p className="muted">No Tier-2 links yet.</p>
            ) : (
              <ul className="occ-list">
                {tier2.map((t) => (
                  <li key={t.id} className="occ">
                    <div className="occ-top">
                      <span className="occ-anchor mono">{t.source_domain}</span>
                      <span className="badge" style={{ "--st": statusColor(t.status) } as CSSProperties}><span className="dot" />{statusLabel(t.status)}</span>
                    </div>
                  </li>
                ))}
              </ul>
            )}
            <div className="tags-add">
              <textarea
                style={{ flex: 1, minHeight: 44, fontFamily: "ui-monospace, monospace", fontSize: 12, background: "var(--bg)", color: "var(--text)", border: "1px solid var(--border)", borderRadius: 6, padding: "6px 8px" }}
                placeholder="source_url,anchor,rel (one per line)"
                value={t2csv}
                onChange={(e) => setT2csv(e.target.value)}
              />
              <button className="btn sm" onClick={() => void importTier2()}>Add T2</button>
            </div>
            {t2msg && <p className="muted" style={{ marginTop: -6, marginBottom: 10 }}>{t2msg}</p>}
            <div className="tags-row">
              {blTags.map((t) => (
                <span key={t.id} className="tag-chip" style={t.color ? { borderColor: t.color, color: t.color } : undefined}>
                  {t.name}
                  <button className="tag-x" onClick={() => void removeTag(t.id)} aria-label={`Remove ${t.name}`}>×</button>
                </span>
              ))}
              {blTags.length === 0 && <span className="muted">No tags</span>}
            </div>
            <div className="tags-add">
              <select
                value=""
                onChange={(e) => { if (e.target.value) void attachExisting(e.target.value); }}
              >
                <option value="">Add tag…</option>
                {orgTags
                  .filter((t) => !blTags.some((b) => b.id === t.id))
                  .map((t) => (
                    <option key={t.id} value={t.id}>{t.name}</option>
                  ))}
              </select>
              <input
                placeholder="new tag"
                value={newTag}
                onChange={(e) => setNewTag(e.target.value)}
                onKeyDown={(e) => { if (e.key === "Enter") void createAndAttach(); }}
              />
              <button className="btn sm" onClick={() => void createAndAttach()}>Add</button>
            </div>
            {me?.features?.gsc && (
              <>
                <h3 className="drawer-sub">
                  Post-Link Impact <span className="impact-beta">BETA</span>
                </h3>
                {impact === null ? (
                  <p className="muted">Loading impact…</p>
                ) : !impact.eligible ? (
                  <div className="impact-empty">
                    <p className="muted">{impact.summary}</p>
                    <button
                      className="btn sm"
                      disabled={impactBusy}
                      onClick={() => void enableImpact()}
                    >
                      {impactBusy ? "Enabling…" : "Track impact"}
                    </button>
                  </div>
                ) : (
                  <div className="impact">
                    <p className="impact-summary">{impact.summary}</p>
                    <div className="impact-meta">
                      {impact.impact_started_at && (
                        <span className="muted mono">
                          Observed since {fmtShort(impact.impact_started_at)}
                        </span>
                      )}
                      {impact.confidence && (
                        <span className={`impact-conf conf-${impact.confidence}`}>
                          {impact.confidence} confidence
                        </span>
                      )}
                    </div>

                    <div className="impact-layers">
                      {(
                        [
                          ["Crawl", impact.crawl.status],
                          ["Indexation", impact.indexation.status],
                          ["Visibility", impact.visibility.status],
                          ["Ranking", impact.ranking.status],
                        ] as const
                      ).map(([name, st]) => (
                        <span
                          key={name}
                          className="impact-chip"
                          style={{ "--sig": signalColor(st) } as CSSProperties}
                        >
                          <span className="dot" />
                          {name}: {signalLabel(st)}
                        </span>
                      ))}
                    </div>

                    {(impact.crawl.last_crawl_time || impact.indexation.coverage_state) && (
                      <div
                        style={{
                          display: "flex",
                          flexDirection: "column",
                          gap: 4,
                          fontSize: 12,
                        }}
                      >
                        {impact.crawl.last_crawl_time && (
                          <div>
                            <strong>Crawl</strong>{" "}
                            <span className="muted">
                              Googlebot last fetched this page{" "}
                              {fmtShort(impact.crawl.last_crawl_time)}
                              {impact.crawl.crawled_since_t0 ? " · since acquisition" : ""}
                            </span>
                          </div>
                        )}
                        {impact.indexation.coverage_state && (
                          <div>
                            <strong>Indexation</strong>{" "}
                            <span className="muted">
                              {impact.indexation.coverage_state}
                              {impact.indexation.canonical_mismatch
                                ? " · Google's canonical differs from yours"
                                : ""}
                            </span>
                          </div>
                        )}
                      </div>
                    )}

                    {impact.visibility.windows.length > 0 && (
                      <div className="impact-windows">
                        {impact.visibility.windows.map((w) => (
                          <div key={w.label} className="impact-window">
                            <div className="iw-head">
                              <strong>{w.label} visibility</strong>
                              <span className="muted sm">{signalLabel(w.status)}</span>
                            </div>
                            {w.status === "pending" ? (
                              <p className="muted sm">
                                Completes on {w.available_on ? fmtShort(w.available_on) : "-"}.
                              </p>
                            ) : (
                              <BeforeAfterChart
                                before={w.impressions_before}
                                after={w.impressions_after}
                                unit="impressions"
                                signal={w.status}
                                changePercent={w.change_percent}
                              />
                            )}
                          </div>
                        ))}
                      </div>
                    )}

                    {impact.ranking.position_before != null &&
                      impact.ranking.position_after != null && (
                        <div className="impact-rank">
                          <strong>Avg. position</strong>
                          <span className="mono">
                            {impact.ranking.position_before.toFixed(1)} →{" "}
                            {impact.ranking.position_after.toFixed(1)}
                          </span>
                          <span className="muted sm">(lower is better)</span>
                        </div>
                      )}

                    {impact.context.overlapping_acquisitions > 0 && (
                      <p className="impact-warn">
                        {impact.context.overlapping_acquisitions} other link(s) to this page
                        were acquired in the same window, changes can&apos;t be isolated to
                        one link.
                      </p>
                    )}

                    <p className="impact-foot muted sm">
                      Observed correlation, not proof of causation.
                    </p>
                  </div>
                )}
              </>
            )}
            {events === null ? (
              <p className="muted">Loading history…</p>
            ) : events.length === 0 ? (
              <p className="muted">No changes recorded yet.</p>
            ) : (
              <ul className="timeline">
                {events.map((e, i) => {
                  const prev = typeof e.previous_data === "string" ? e.previous_data : null;
                  const curr = typeof e.current_data === "string" ? e.current_data : null;
                  return (
                    <li key={i}>
                      <span className="tl-dot" />
                      <div>
                        <div className="tl-type">{eventLabel(e.event_type)}</div>
                        {(prev || curr) && (
                          <div className="tl-diff">
                            {prev && <span className="tl-before">{prev}</span>}
                            {prev && curr && <span className="tl-arrow"> → </span>}
                            {curr && <span className="tl-after">{curr}</span>}
                          </div>
                        )}
                        <div className="muted mono">{new Date(e.detected_at).toLocaleString()}</div>
                      </div>
                    </li>
                  );
                })}
              </ul>
            )}
          </aside>
        </div>
      )}
    </main>
  );
}

function fmtShort(s?: string | null): string {
  if (!s) return "-";
  const d = new Date(s);
  return Number.isNaN(d.getTime()) ? "-" : d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}
// Post-Link Impact signal → token color / human label (mirrors the Rust SignalState).
function signalColor(st: string): string {
  switch (st) {
    case "positive":
      return "var(--live)";
    case "negative":
      return "var(--lost)";
    case "neutral":
      return "var(--changed)";
    case "pending":
      return "var(--pending)";
    default:
      return "var(--unknown)";
  }
}
function signalLabel(st: string): string {
  switch (st) {
    case "positive":
      return "Positive";
    case "negative":
      return "Negative";
    case "neutral":
      return "No clear change";
    case "pending":
      return "Collecting";
    case "insufficient_data":
      return "Not enough data";
    default:
      return "Unavailable";
  }
}
function fmtLong(s?: string | null): string {
  if (!s) return "-";
  const d = new Date(s);
  return Number.isNaN(d.getTime()) ? "-" : d.toLocaleString();
}
function httpPair(b: BacklinkView): string {
  const s = b.source_http_status ?? null;
  const t = b.target_http_status ?? null;
  if (s == null && t == null) return "-";
  return `${s ?? "-"} / ${t ?? "-"}`;
}
function yn(v?: boolean | null): string {
  return v == null ? "-" : v ? "yes" : "no";
}
// Compact distribution donut used in the 3-up stats row (follow ratio / status
// / link type). Conic-gradient ring, center shows the headline share, legend
// below. Dependency-free, themeable.
function MiniDonut({
  title,
  head,
  segments,
  center,
}: {
  title: string;
  head: ReactNode;
  segments: { key: string; label: string; n: number; color: string }[];
  center: { value: string; label: string };
}) {
  const total = segments.reduce((a, s) => a + s.n, 0);
  let acc = 0;
  const stops = segments
    .filter((s) => s.n > 0)
    .map((s) => {
      const from = (acc / total) * 360;
      acc += s.n;
      const to = (acc / total) * 360;
      return `${s.color} ${from}deg ${to}deg`;
    })
    .join(", ");
  return (
    <section className="dist">
      <div className="dist-head">
        <h2>{title}</h2>
        <span className="dist-total">{head}</span>
      </div>
      <div className="mini-donut">
        <div
          className="mini-ring"
          style={{ background: `conic-gradient(${stops})` }}
          role="img"
          aria-label={title}
        >
          <span className="mini-ctr">
            <b>{center.value}</b>
            <span>{center.label}</span>
          </span>
        </div>
        <div className="legend">
          {segments.map((s) => (
            <span key={s.key} className="legend-item" style={{ color: s.color }}>
              <span className="legend-dot" />
              <span style={{ color: "var(--muted)" }}>{s.label}</span>
              <b>{s.n}</b>
              <span style={{ color: "var(--muted)" }}>
                ({total ? Math.round((s.n / total) * 100) : 0}%)
              </span>
            </span>
          ))}
        </div>
      </div>
    </section>
  );
}

function RelChips({ b }: { b: BacklinkView }) {
  const rel = b.detected_rel;
  if (!rel || rel.length === 0) {
    return <span className="rel-chip rel-expected" title="Expected (not yet observed)">{b.expected_rel_type}</span>;
  }
  return (
    <span className="rel-chips">
      {rel.map((r) => (
        <span key={r} className={`rel-chip rel-${r}`}>{r}</span>
      ))}
    </span>
  );
}


// A KPI tile for the command-center row. Reuses the shared dashboard tile design
// (.tile / .kpi-accent / .delta-pill from overview.css) so it matches /dashboard.
function BlKpi({
  label,
  value,
  hint,
  icon,
  accent,
  delta,
  onClick,
}: {
  label: string;
  value: string;
  hint: string;
  icon: string;
  accent?: string;
  delta?: number | null;
  onClick?: () => void;
}) {
  const dir = delta == null ? null : delta > 0 ? "up" : delta < 0 ? "down" : "flat";
  const style = accent ? ({ "--tile-accent": accent } as CSSProperties) : undefined;
  return (
    <button type="button" className="tile kpi-accent kpi-link" onClick={onClick} style={style}>
      <div className="tile-top">
        <span className="tile-label">{label}</span>
        <span className="tile-icon">
          <Icon name={icon} size={16} />
        </span>
      </div>
      <div className="tile-value" style={accent ? { color: accent } : undefined}>
        {value}
      </div>
      <div className="tile-foot">
        {dir && (
          <span className={`delta-pill ${dir}`}>
            {dir === "up" ? "↑" : dir === "down" ? "↓" : "→"} {Math.abs(delta!).toFixed(1)}%
          </span>
        )}
        <span className="tile-hint">{hint}</span>
      </div>
    </button>
  );
}

// Human labels for backlink_events timeline entries (clearer than the raw
// slugs). Keep in sync with EventType in crates/core/src/event.rs.
const EVENT_LABELS: Record<string, string> = {
  backlink_found: "Backlink found",
  backlink_lost: "Backlink lost",
  backlink_restored: "Backlink restored",
  anchor_changed: "Anchor changed",
  target_changed: "Target URL changed",
  dofollow_to_nofollow: "Became nofollow",
  nofollow_to_dofollow: "Became dofollow",
  rel_changed: "Rel attribute changed",
  sponsored_added: "Marked sponsored",
  sponsored_removed: "Sponsored removed",
  ugc_added: "Marked UGC",
  ugc_removed: "UGC removed",
  link_moved: "Link moved on the page",
  source_redirected: "Source page redirected",
  source_error: "Source page error",
  target_redirected: "Target URL redirected",
  target_error: "Target URL error",
  noindex_detected: "Source page noindexed",
  canonical_changed: "Canonical changed",
  competitor_new_backlink: "Competitor: new backlink",
  competitor_new_referring_domain: "Competitor: new referring domain",
  competitor_new_referring_page: "Competitor: new referring page",
};

function eventLabel(eventType: string): string {
  return EVENT_LABELS[eventType] ?? eventType.replace(/_/g, " ");
}
