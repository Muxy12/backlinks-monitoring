// Thin client for the LinkPulz API. Cookies carry the session, so every request
// uses `credentials: "include"`. Base URL is configured per environment.
import type { Attribution } from "./attribution";

const BASE = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";

export type ApiResult<T> = { ok: true; data: T } | { ok: false; status: number; detail: string };

async function request<T>(path: string, init?: RequestInit): Promise<ApiResult<T>> {
  try {
    const res = await fetch(`${BASE}${path}`, {
      ...init,
      credentials: "include",
      headers: { "Content-Type": "application/json", ...(init?.headers ?? {}) },
    });
    const text = await res.text();
    const body = text ? JSON.parse(text) : null;
    if (!res.ok) {
      const detail = (body && (body.detail as string)) || res.statusText;
      return { ok: false, status: res.status, detail };
    }
    return { ok: true, data: body as T };
  } catch (e) {
    return { ok: false, status: 0, detail: e instanceof Error ? e.message : "network error" };
  }
}

export interface UserView {
  id: string;
  email: string;
  name: string;
  email_verified: boolean;
  is_admin?: boolean;
}
export interface OrgView {
  id: string;
  name: string;
  slug: string;
  role: string;
}
export interface Features {
  campaigns_beta: boolean;
  gsc?: boolean;
}
export interface GscStatus {
  configured: boolean;
  connected: boolean;
  site_url: string | null;
  google_email: string | null;
  last_sync_at: string | null;
}
export interface GscSite {
  site_url: string;
  permission_level: string | null;
}
// Post-Link Impact (BETA) — observed before/after around a backlink's T0.
// `status` values: positive | neutral | negative | insufficient_data | unavailable | pending.
export interface ImpactLayerStub {
  status: string;
  note: string | null;
}
export interface ImpactVisibilityWindow {
  label: string; // "7d" | "28d"
  status: string;
  available_on: string | null; // set while the window is still "pending"
  impressions_before: number;
  impressions_after: number;
  clicks_before: number;
  clicks_after: number;
  change_percent: number | null; // null when the baseline was 0 (new activity)
}
export interface ImpactVisibilityLayer {
  status: string;
  windows: ImpactVisibilityWindow[];
}
export interface ImpactRankingLayer {
  status: string;
  position_before: number | null;
  position_after: number | null;
  position_change: number | null; // negative = improved (lower is better)
}
export interface ImpactTimelineItem {
  kind: string; // "t0" | backlink_* event types
  at: string;
  label: string;
}
export interface ImpactContext {
  overlapping_acquisitions: number; // other links to the same page in the window (§44)
}
export interface ImpactView {
  status: string;
  eligible: boolean;
  impact_started_at: string | null; // T0
  t0_source: string | null; // acquisition | verification | manual
  data_sources: string[];
  crawl: ImpactLayerStub; // "unavailable" in BETA
  indexation: ImpactLayerStub; // "unavailable" in BETA
  visibility: ImpactVisibilityLayer;
  ranking: ImpactRankingLayer;
  timeline: ImpactTimelineItem[];
  context: ImpactContext;
  confidence: string | null; // low | medium | high
  summary: string;
}
export interface MeResponse {
  user: UserView;
  organizations: OrgView[];
  features?: Features;
}

export interface BacklinkCheckResult {
  found: boolean;
  http_status: number | null;
  final_url: string | null;
  anchor_text: string | null;
  rel: string[] | null;
  is_dofollow: boolean | null;
  source_noindex: boolean | null;
  error: string | null;
}

// The plan catalog, served from the backend's single source of truth (§39).
// `backlinks`/`projects`/`seats` are null when unlimited.
export interface PlanView {
  slug: string;
  name: string;
  price_monthly_cents: number;
  price_annual_cents: number;
  backlinks: number | null;
  projects: number | null;
  seats: number | null;
  monitoring_interval_hours: number;
  most_popular: boolean;
}

/** One usage dimension against the plan's cap (`limit: null` = unlimited). */
export interface UsageItem {
  used: number;
  limit: number | null;
}

export interface UsageView {
  plan: string;
  backlinks: UsageItem;
  projects: UsageItem;
  seats: UsageItem;
}

// A user feedback submission (feature request or bug report).
export interface FeedbackItem {
  id: string;
  kind: string; // "feature" | "bug"
  title: string;
  body: string;
  page_url: string | null;
  status: string; // open | planned | in_progress | done | declined
  created_at: string;
  user_email: string;
}

export const api = {
  // Public plan catalog for the pricing + billing pages (no auth).
  plans: () => request<PlanView[]>("/v1/plans"),
  // Pre-launch waitlist email capture (public, no session).
  joinWaitlist: (email: string, turnstileToken?: string, attribution?: Attribution) =>
    request<{ ok: boolean; already: boolean }>("/v1/waitlist", {
      method: "POST",
      body: JSON.stringify({ email, turnstile_token: turnstileToken, attribution }),
    }),
  waitlistCount: () => request<{ count: number }>("/v1/waitlist/count"),
  unsubscribeWaitlist: (token: string) =>
    request<{ ok: boolean; removed: boolean }>("/v1/waitlist/unsubscribe", {
      method: "POST",
      body: JSON.stringify({ token }),
    }),
  register: (
    email: string,
    password: string,
    name: string,
    turnstileToken?: string,
    attribution?: Attribution,
  ) =>
    request<UserView>("/v1/auth/register", {
      method: "POST",
      body: JSON.stringify({ email, password, name, turnstile_token: turnstileToken, attribution }),
    }),
  login: (email: string, password: string) =>
    request<{ user: UserView }>("/v1/auth/login", { method: "POST", body: JSON.stringify({ email, password }) }),
  logout: () => request<null>("/v1/auth/logout", { method: "POST" }),
  verifyEmail: (token: string) =>
    request<null>("/v1/auth/verify", { method: "POST", body: JSON.stringify({ token }) }),
  resendVerification: (email: string) =>
    request<null>("/v1/auth/resend-verification", {
      method: "POST",
      body: JSON.stringify({ email }),
    }),
  forgotPassword: (email: string, turnstileToken?: string) =>
    request<null>("/v1/auth/forgot", {
      method: "POST",
      body: JSON.stringify({ email, turnstile_token: turnstileToken }),
    }),
  resetPassword: (token: string, password: string) =>
    request<null>("/v1/auth/reset", {
      method: "POST",
      body: JSON.stringify({ token, password }),
    }),
  me: () => request<MeResponse>("/v1/me"),
  // Submit a feature request or bug report (authenticated).
  submitFeedback: (f: {
    kind: string;
    title: string;
    body: string;
    page_url?: string | null;
    organization_id?: string | null;
  }) =>
    request<FeedbackItem>("/v1/feedback", {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify(f),
    }),
  // The caller's own submissions, newest first.
  myFeedback: () => request<FeedbackItem[]>("/v1/feedback/mine"),
  // Create a workspace (organization). Slug is derived from the name server-side
  // when omitted. The caller is made the owner.
  createOrganization: (name: string, slug?: string) =>
    request<OrgView>("/v1/organizations", {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify(slug ? { name, slug } : { name }),
    }),
  acceptInvitation: (token: string) =>
    request<{ organization_id: string; role: string }>("/v1/invitations/accept", {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify({ token }),
    }),
  // Public, no auth (§50).
  checkBacklink: (sourceUrl: string, targetUrl: string, turnstileToken?: string) =>
    request<BacklinkCheckResult>("/v1/tools/backlink-check", {
      method: "POST",
      body: JSON.stringify({
        source_url: sourceUrl,
        target_url: targetUrl,
        turnstile_token: turnstileToken,
      }),
    }),
};

export interface DashboardResponse {
  counts: Record<string, number>;
  monitored: number;
  needs_attention: number;
  protected?: number;
  tracked?: number;
  inventory_only?: number;
  value_protected?: { currency: string; amount: string }[];
  value_at_risk: { currency: string; amount: string }[];
  tracked_spend: { currency: string; amount: string }[];
  referring_domains: number;
  referring_pages: number;
  occurrences: number;
  rel: number[];
  link_types: Record<string, number>;
}

export interface SeedView {
  id: string;
  domain: string;
  normalized_domain: string;
  source: string;
  status: string;
  pages_crawled: number;
  backlinks_found: number;
  last_crawled_at: string | null;
  next_crawl_at: string;
}

export interface GraphNode {
  id: string;
  label: string;
  domain: string;
  tier: number;
  type: string;
}
export interface GraphEdge {
  source: string;
  target: string;
  backlink_id: string;
  tier: number;
  status: string;
}
export interface GraphResponse {
  view: string;
  root: string;
  nodes: GraphNode[];
  edges: GraphEdge[];
  truncated: boolean;
}

export interface CompetitorView {
  id: string;
  project_id: string;
  domain: string;
  normalized_domain: string;
  enabled: boolean;
  first_scanned_at: string | null;
  last_scanned_at: string | null;
}

export interface EventsSummary {
  since: string;
  days: number;
  counts: Record<string, number>;
}

export interface OccurrenceView {
  id: string;
  fingerprint: string;
  anchor_text: string | null;
  link_type: string | null;
  image_alt: string | null;
  href_raw: string | null;
  rel: string[] | null;
  is_nofollow: boolean;
  is_sponsored: boolean;
  is_ugc: boolean;
  placement: string;
  surrounding_text: string | null;
  position: number | null;
  first_seen_at: string;
  last_seen_at: string;
}
export interface ProjectView {
  id: string;
  name: string;
  domain: string;
  check_frequency_secs: number;
  status: string;
  client_id: string | null;
}
export interface BacklinkView {
  id: string;
  source_url: string;
  target_url: string;
  source_domain: string;
  target_domain: string;
  expected_anchor_text: string | null;
  expected_rel_type: string;
  status: string;
  cost: string | null;
  currency: string | null;
  warranty_type?: string;
  warranty_value?: number | null;
  warranty_expiration?: string | null;
  warranty_status?: string;
  // Observed on the last crawl (docs/06). Absent until first checked.
  detected_anchor_text?: string | null;
  detected_target_url?: string | null;
  detected_rel?: string[] | null;
  is_dofollow?: boolean | null;
  is_nofollow?: boolean | null;
  is_sponsored?: boolean | null;
  is_ugc?: boolean | null;
  source_http_status?: number | null;
  target_http_status?: number | null;
  first_checked_at?: string | null;
  last_seen_at?: string | null;
  last_checked_at?: string | null;
  link_type?: string | null;
  image_alt?: string | null;
  lost_at?: string | null;
  target_final_url?: string | null;
  tier?: number;
  parent_backlink_id?: string | null;
  // Manual classification (slugs; null = unclassified).
  acquisition?: string | null;
  placement?: string | null;
  link_format?: string | null;
  /// Manual attribute override (null = use link_attribute_auto).
  link_attribute?: string | null;
  /// Crawler-derived attribute; null if never observed.
  link_attribute_auto?: string | null;
  // Selective monitoring (migration 0035).
  monitoring_enabled?: boolean;
  monitoring_level?: string;
  check_frequency?: string | null;
  monitoring_reason?: string | null;
  monitoring_overridden?: boolean;
  next_check_at?: string | null;
  // Economic / commercial.
  commercial_model?: string | null;
  order_reference?: string | null;
  purchase_date?: string | null;
  renewal_date?: string | null;
  replacement_eligible?: boolean | null;
}

export interface NotificationItem {
  id: string;
  kind: string;
  title: string;
  body: string | null;
  backlink_id: string | null;
  read: boolean;
  created_at: string;
}
export interface NotificationsResponse {
  items: NotificationItem[];
  unread: number;
}

// A direct URL to the CSV export. It is a top-level GET navigation (an <a>
// download), so the SameSite=Lax session cookie is sent — no fetch needed.
export function backlinksExportUrl(org: string, projectId?: string, clientId?: string): string {
  const base = `${BASE}/v1/organizations/${org}/backlinks/export`;
  const p = new URLSearchParams();
  if (clientId) p.set("client_id", clientId);
  else if (projectId) p.set("project_id", projectId);
  const qs = p.toString();
  return qs ? `${base}?${qs}` : base;
}

// The CSRF cookie is not httpOnly (double-submit), so echo it on mutations.
function csrfHeader(): Record<string, string> {
  if (typeof document === "undefined") return {};
  const m = document.cookie.match(/(?:^|;\s*)csrf=([^;]+)/);
  return m ? { "x-csrf-token": decodeURIComponent(m[1]) } : {};
}

export interface MoneyByCurrency {
  currency: string;
  amount: string;
}
export interface CampaignView {
  id: string;
  project_id: string;
  name: string;
  description: string | null;
  status: string;
  target_backlinks: number | null;
  budget: string | null;
  currency: string | null;
  // Link-building fields (migration 0036). Dates are ISO YYYY-MM-DD strings.
  objective?: string | null;
  primary_target_url?: string | null;
  start_date?: string | null;
  end_date?: string | null;
}

// ─── Link-building campaigns (BETA, migration 0036) ──────────────────────────
export interface CampaignTargetView {
  id: string;
  campaign_id: string;
  url: string;
  label: string | null;
}
export interface ProspectView {
  id: string;
  campaign_id: string;
  domain: string;
  stage: string;
  source: string;
  score: number | null;
  score_reason: string | null;
  competitor_count: number | null;
  contact_name: string | null;
  contact_email: string | null;
  target_url: string | null;
  estimated_cost: string | null;
  currency: string | null;
  notes: string | null;
  last_contacted_at: string | null;
  won_backlink_id: string | null;
}
export interface StageCount {
  stage: string;
  count: number;
}
export interface CampaignSummary {
  campaign_id: string;
  total: number;
  live: number;
  lost: number;
  needs_attention: number;
  removal_rate: number;
  target_backlinks: number | null;
  target_progress: number | null;
  budget: string | null;
  currency: string | null;
  spend: MoneyByCurrency[];
  prospects_total: number;
  stage_counts: StageCount[];
}
export interface OutreachDraft {
  subject: string;
  body: string;
}

// ─── Overview / command center (dashboard redesign) ──────────────────────────
export interface OverviewLinkHealth {
  percentage: number | null;
  healthy: number;
  eligible: number;
  change: number | null;
}
export interface OverviewActiveCampaigns {
  active: number;
  draft: number;
  total: number;
}
export interface OverviewSummary {
  link_health: OverviewLinkHealth;
  protected_value: MoneyByCurrency[];
  active_campaigns: OverviewActiveCampaigns;
  value_at_risk: MoneyByCurrency[];
}
export interface AttentionItemView {
  priority: string; // critical | high | medium | low
  kind: string;
  title: string;
  subtitle: string | null;
  value_at_risk: MoneyByCurrency | null;
  action: string;
  route: string;
}
export interface GrowthPoint {
  day: string;
  referring_domains: number;
  protected: number;
  live: number;
}
export interface OverviewBacklinkGrowth {
  series: GrowthPoint[];
  new_count: number;
  lost_count: number;
  net_count: number;
}
export interface OverviewCampaignPerformance {
  active: number;
  budget: MoneyByCurrency[];
  spend: MoneyByCurrency[];
  prospects_total: number;
  stage_counts: StageCount[];
}
export interface OverviewOpportunity {
  prospect_id: string;
  campaign_id: string;
  domain: string;
  score: number | null;
  score_reason: string | null;
  estimated_cost: string | null;
  currency: string | null;
}
export interface OverviewCompetitive {
  our_referring_domains: number;
  competitor_referring_domains: number;
  biggest_gap_domain: string | null;
  biggest_gap_competitors: number | null;
}
export interface OverviewActivity {
  event_type: string;
  source_domain: string;
  target_url: string;
  backlink_id: string;
  detected_at: string;
}
export interface OverviewResponse {
  summary: OverviewSummary;
  attention: AttentionItemView[];
  next_best_action: AttentionItemView | null;
  backlink_growth: OverviewBacklinkGrowth;
  campaign_performance: OverviewCampaignPerformance | null;
  top_opportunities: OverviewOpportunity[];
  competitive_position: OverviewCompetitive | null;
  recent_activity: OverviewActivity[];
  generated_at: string;
}
export interface NewProspect {
  domain: string;
  contact_name?: string;
  contact_email?: string;
  target_url?: string;
  estimated_cost?: number;
  currency?: string;
  notes?: string;
}
export interface CampaignRollup {
  campaign_id: string;
  total: number;
  live: number;
  lost: number;
  needs_attention: number;
  removal_rate: number;
  avg_lifespan_days: number | null;
  target_backlinks: number | null;
  target_progress: number | null;
  spend: MoneyByCurrency[];
}
export interface PublisherView {
  id: string;
  name: string;
  domain: string;
  contact_name: string | null;
  contact_email: string | null;
}
export interface PublisherRollup {
  publisher_id: string;
  total: number;
  live: number;
  lost: number;
  needs_attention: number;
  removal_rate: number;
  avg_lifespan_days: number | null;
  spend: MoneyByCurrency[];
}
export interface NewCampaign {
  project_id: string;
  name: string;
  description?: string;
  target_backlinks?: number;
  budget?: number;
  currency?: string;
  objective?: string;
  primary_target_url?: string;
  start_date?: string;
  end_date?: string;
}
export interface NewPublisher {
  name: string;
  domain: string;
  contact_name?: string;
  contact_email?: string;
}
export interface ClientView {
  id: string;
  name: string;
  company_name: string | null;
  website: string | null;
  contact_name: string | null;
  contact_email: string | null;
}
export interface NewClient {
  name: string;
  company_name?: string;
  website?: string;
  contact_name?: string;
  contact_email?: string;
  notes?: string;
}
export interface TagView {
  id: string;
  name: string;
  color: string | null;
}
export interface MemberView {
  user_id: string;
  email: string;
  name: string;
  role: string;
}
export interface InvitationView {
  id: string;
  email: string;
  role: string;
  expires_at: string;
  token?: string;
}
export interface PreferenceItem {
  event_type: string;
  channel: string;
  enabled: boolean;
  digest?: string;
}
export interface AlertDestination {
  id: string;
  kind: string;
  target: string;
  name: string;
  enabled: boolean;
  digest: string;
  last_status: string | null;
  last_error: string | null;
  last_delivered_at: string | null;
  created_at: string;
}

export const orgApi = {
  dashboard: (org: string, days?: number) =>
    request<DashboardResponse>(
      `/v1/organizations/${org}/dashboard${days ? `?days=${days}` : ""}`,
    ),
  overview: (org: string, range?: string) =>
    request<OverviewResponse>(
      `/v1/organizations/${org}/overview${range ? `?range=${range}` : ""}`,
    ),
  // ── Google Search Console (Post-Link Impact foundation) ──
  gscStatus: (org: string, projectId: string) =>
    request<GscStatus>(`/v1/organizations/${org}/projects/${projectId}/gsc/status`),
  gscAuthUrl: (org: string, projectId: string) =>
    request<{ url: string }>(`/v1/organizations/${org}/projects/${projectId}/gsc/auth-url`),
  gscSites: (org: string, projectId: string) =>
    request<GscSite[]>(`/v1/organizations/${org}/projects/${projectId}/gsc/sites`),
  gscSetSite: (org: string, projectId: string, siteUrl: string) =>
    request<null>(`/v1/organizations/${org}/projects/${projectId}/gsc/site`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify({ site_url: siteUrl }),
    }),
  gscDisconnect: (org: string, projectId: string) =>
    request<null>(`/v1/organizations/${org}/projects/${projectId}/gsc`, {
      method: "DELETE",
      headers: csrfHeader(),
    }),
  projects: (org: string) => request<ProjectView[]>(`/v1/organizations/${org}/projects`),
  createProject: (
    org: string,
    body: { name: string; domain: string; check_frequency_hours?: number; client_id?: string },
  ) =>
    request<ProjectView>(`/v1/organizations/${org}/projects`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify(body),
    }),
  updateProject: (
    org: string,
    id: string,
    body: { name?: string; check_frequency_hours?: number; client_id?: string | null },
  ) =>
    request<ProjectView>(`/v1/organizations/${org}/projects/${id}`, {
      method: "PATCH",
      headers: csrfHeader(),
      body: JSON.stringify(body),
    }),
  deleteProject: (org: string, id: string) =>
    request<null>(`/v1/organizations/${org}/projects/${id}`, {
      method: "DELETE",
      headers: csrfHeader(),
    }),
  autoTagBacklinks: (org: string, projectId: string) =>
    request<{ total: number; counts: Record<string, number> }>(
      `/v1/organizations/${org}/projects/${projectId}/auto-tag`,
      { method: "POST", headers: csrfHeader() },
    ),
  backlinks: (org: string, projectId: string) =>
    request<BacklinkView[]>(`/v1/organizations/${org}/backlinks?project_id=${projectId}`),
  searchBacklinks: (
    org: string,
    opts: {
      projectId?: string;
      clientId?: string;
      status?: string;
      q?: string;
      tagId?: string;
      limit?: number;
      offset?: number;
      days?: number;
      acquisition?: string;
      attribute?: string;
      placement?: string;
      format?: string;
      monitoringLevel?: string;
    },
  ) => {
    const p = new URLSearchParams();
    if (opts.projectId) p.set("project_id", opts.projectId);
    if (opts.clientId) p.set("client_id", opts.clientId);
    if (opts.status) p.set("status", opts.status);
    if (opts.q) p.set("q", opts.q);
    if (opts.tagId) p.set("tag_id", opts.tagId);
    if (opts.limit != null) p.set("limit", String(opts.limit));
    if (opts.offset != null) p.set("offset", String(opts.offset));
    if (opts.days) p.set("days", String(opts.days));
    if (opts.acquisition) p.set("acquisition", opts.acquisition);
    if (opts.attribute) p.set("attribute", opts.attribute);
    if (opts.placement) p.set("placement", opts.placement);
    if (opts.format) p.set("format", opts.format);
    if (opts.monitoringLevel) p.set("monitoring_level", opts.monitoringLevel);
    const qs = p.toString();
    return request<BacklinkView[]>(
      `/v1/organizations/${org}/backlinks${qs ? `?${qs}` : ""}`,
    );
  },
  backlinkEvents: (org: string, id: string) =>
    request<EventView[]>(`/v1/organizations/${org}/backlinks/${id}/events`),
  backlinkOccurrences: (org: string, id: string) =>
    request<OccurrenceView[]>(`/v1/organizations/${org}/backlinks/${id}/occurrences`),
  backlinkImpact: (org: string, id: string) =>
    request<ImpactView>(`/v1/organizations/${org}/backlinks/${id}/impact`),
  enableBacklinkImpact: (org: string, id: string) =>
    request<null>(`/v1/organizations/${org}/backlinks/${id}/impact/enable`, {
      method: "POST",
      headers: csrfHeader(),
    }),
  backlinkTier2: (org: string, id: string) =>
    request<BacklinkView[]>(`/v1/organizations/${org}/backlinks/${id}/tier2`),
  importTier2: (
    org: string,
    id: string,
    rows: { source_url: string; anchor?: string; rel?: string }[],
  ) =>
    request<{ imported: number; new: number; skipped: number }>(
      `/v1/organizations/${org}/backlinks/${id}/tier2/import`,
      { method: "POST", headers: csrfHeader(), body: JSON.stringify({ rows }) },
    ),
  eventsSummary: (org: string, days = 30) =>
    request<EventsSummary>(`/v1/organizations/${org}/events/summary?days=${days}`),
  competitors: (org: string, projectId: string) =>
    request<CompetitorView[]>(`/v1/organizations/${org}/projects/${projectId}/competitors`),
  createCompetitor: (org: string, projectId: string, domain: string) =>
    request<CompetitorView>(`/v1/organizations/${org}/projects/${projectId}/competitors`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify({ domain }),
    }),
  deleteCompetitor: (org: string, competitorId: string) =>
    request<null>(`/v1/organizations/${org}/competitors/${competitorId}`, {
      method: "DELETE",
      headers: csrfHeader(),
    }),
  competitorBacklinks: (org: string, competitorId: string) =>
    request<BacklinkView[]>(`/v1/organizations/${org}/competitors/${competitorId}/backlinks`),
  discoverySeeds: (org: string, projectId: string) =>
    request<SeedView[]>(`/v1/organizations/${org}/projects/${projectId}/discovery/seeds`),
  runDiscovery: (
    org: string,
    projectId: string,
    provider: string,
    competitorId?: string,
    limit = 200,
  ) =>
    request<{ provider: string; discovered: number; new: number }>(
      `/v1/organizations/${org}/projects/${projectId}/discovery/run`,
      {
        method: "POST",
        headers: csrfHeader(),
        body: JSON.stringify({ provider, competitor_id: competitorId ?? null, limit }),
      },
    ),
  importSeeds: (org: string, projectId: string, domains: string[], source = "gsc") =>
    request<{ added: number; skipped: number }>(
      `/v1/organizations/${org}/projects/${projectId}/discovery/seeds`,
      { method: "POST", headers: csrfHeader(), body: JSON.stringify({ domains, source }) },
    ),
  backlinkGraph: (org: string, projectId: string, view: "domain" | "url" = "domain", limit = 2000) =>
    request<GraphResponse>(
      `/v1/organizations/${org}/projects/${projectId}/backlink-graph?view=${view}&limit=${limit}`,
    ),
  backlinkGap: (org: string, projectId: string, min = 1) =>
    request<{ source_domain: string; competitor_count: number }[]>(
      `/v1/organizations/${org}/projects/${projectId}/backlink-gap?min=${min}`,
    ),
  importCompetitorBacklinks: (
    org: string,
    competitorId: string,
    rows: { source_url: string; target_url: string; anchor?: string; rel?: string }[],
  ) =>
    request<{ imported: number; new: number; skipped: number }>(
      `/v1/organizations/${org}/competitors/${competitorId}/backlinks/import`,
      { method: "POST", headers: csrfHeader(), body: JSON.stringify({ rows }) },
    ),
  archiveBacklink: (org: string, id: string) =>
    request<null>(`/v1/organizations/${org}/backlinks/${id}/archive`, {
      method: "POST",
      headers: csrfHeader(),
    }),
  expiringWarranties: (org: string) =>
    request<BacklinkView[]>(`/v1/organizations/${org}/warranties/expiring`),
  notifications: (org: string) =>
    request<NotificationsResponse>(`/v1/organizations/${org}/notifications`),
  markNotificationRead: (org: string, id: string) =>
    request<null>(`/v1/organizations/${org}/notifications/${id}/read`, {
      method: "POST",
      headers: csrfHeader(),
    }),
  campaigns: (org: string) => request<CampaignView[]>(`/v1/organizations/${org}/campaigns`),
  campaign: (org: string, id: string) =>
    request<CampaignView>(`/v1/organizations/${org}/campaigns/${id}`),
  createCampaign: (org: string, body: NewCampaign) =>
    request<CampaignView>(`/v1/organizations/${org}/campaigns`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify(body),
    }),
  updateCampaign: (
    org: string,
    id: string,
    body: {
      name?: string;
      description?: string;
      status?: string;
      target_backlinks?: number;
      budget?: number;
      currency?: string;
      objective?: string;
      primary_target_url?: string;
      start_date?: string;
      end_date?: string;
    },
  ) =>
    request<CampaignView>(`/v1/organizations/${org}/campaigns/${id}`, {
      method: "PATCH",
      headers: csrfHeader(),
      body: JSON.stringify(body),
    }),
  deleteCampaign: (org: string, id: string) =>
    request<null>(`/v1/organizations/${org}/campaigns/${id}`, {
      method: "DELETE",
      headers: csrfHeader(),
    }),
  campaignRollup: (org: string, id: string) =>
    request<CampaignRollup>(`/v1/organizations/${org}/campaigns/${id}/rollup`),
  // ── Link-building campaigns BETA (migration 0036) ──
  campaignSummary: (org: string, id: string) =>
    request<CampaignSummary>(`/v1/organizations/${org}/campaigns/${id}/summary`),
  campaignTargets: (org: string, id: string) =>
    request<CampaignTargetView[]>(`/v1/organizations/${org}/campaigns/${id}/targets`),
  createCampaignTarget: (org: string, id: string, body: { url: string; label?: string }) =>
    request<CampaignTargetView>(`/v1/organizations/${org}/campaigns/${id}/targets`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify(body),
    }),
  deleteCampaignTarget: (org: string, targetId: string) =>
    request<null>(`/v1/organizations/${org}/campaign-targets/${targetId}`, {
      method: "DELETE",
      headers: csrfHeader(),
    }),
  prospects: (org: string, campaignId: string, stage?: string) =>
    request<ProspectView[]>(
      `/v1/organizations/${org}/campaigns/${campaignId}/prospects${stage ? `?stage=${stage}` : ""}`,
    ),
  createProspect: (org: string, campaignId: string, body: NewProspect) =>
    request<ProspectView>(`/v1/organizations/${org}/campaigns/${campaignId}/prospects`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify(body),
    }),
  updateProspect: (
    org: string,
    prospectId: string,
    body: {
      stage?: string;
      contact_name?: string;
      contact_email?: string;
      target_url?: string;
      estimated_cost?: number;
      currency?: string;
      notes?: string;
    },
  ) =>
    request<ProspectView>(`/v1/organizations/${org}/prospects/${prospectId}`, {
      method: "PATCH",
      headers: csrfHeader(),
      body: JSON.stringify(body),
    }),
  deleteProspect: (org: string, prospectId: string) =>
    request<null>(`/v1/organizations/${org}/prospects/${prospectId}`, {
      method: "DELETE",
      headers: csrfHeader(),
    }),
  bulkProspectStage: (org: string, campaignId: string, ids: string[], stage: string) =>
    request<{ updated: number }>(
      `/v1/organizations/${org}/campaigns/${campaignId}/prospects/bulk-stage`,
      { method: "POST", headers: csrfHeader(), body: JSON.stringify({ ids, stage }) },
    ),
  seedProspectsFromGap: (org: string, campaignId: string, min = 1) =>
    request<{ created: number }>(
      `/v1/organizations/${org}/campaigns/${campaignId}/prospects/from-gap`,
      { method: "POST", headers: csrfHeader(), body: JSON.stringify({ min }) },
    ),
  prospectDraft: (org: string, prospectId: string, senderName?: string) =>
    request<OutreachDraft>(`/v1/organizations/${org}/prospects/${prospectId}/draft`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify({ sender_name: senderName }),
    }),
  convertProspect: (
    org: string,
    prospectId: string,
    body: {
      source_url: string;
      target_url?: string;
      publisher_id?: string;
      cost?: number;
      currency?: string;
      anchor?: string;
      rel?: string;
    },
  ) =>
    request<{ prospect: ProspectView; backlink_id: string }>(
      `/v1/organizations/${org}/prospects/${prospectId}/convert`,
      { method: "POST", headers: csrfHeader(), body: JSON.stringify(body) },
    ),
  publishers: (org: string) => request<PublisherView[]>(`/v1/organizations/${org}/publishers`),
  createPublisher: (org: string, body: NewPublisher) =>
    request<PublisherView>(`/v1/organizations/${org}/publishers`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify(body),
    }),
  publisherRollup: (org: string, id: string) =>
    request<PublisherRollup>(`/v1/organizations/${org}/publishers/${id}/rollup`),
  createBacklink: (
    org: string,
    body: {
      project_id: string;
      source_url: string;
      target_url: string;
      expected_rel_type?: string;
      expected_anchor_text?: string;
      cost?: number;
      currency?: string;
    },
  ) =>
    request<BacklinkView>(`/v1/organizations/${org}/backlinks`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify(body),
    }),
  // Edit a backlink's mutable fields (PATCH). Pass only the fields to change; a
  // classification field set to null clears it (back to unclassified / auto).
  updateBacklink: (
    org: string,
    id: string,
    body: {
      acquisition?: string | null;
      link_attribute?: string | null;
      placement?: string | null;
      link_format?: string | null;
      expected_rel_type?: string;
      expected_anchor_text?: string | null;
      cost?: number | null;
      currency?: string | null;
      monitoring_enabled?: boolean;
      monitoring_level?: string;
      check_frequency?: string | null;
      commercial_model?: string | null;
      order_reference?: string | null;
      purchase_date?: string | null;
      renewal_date?: string | null;
      replacement_eligible?: boolean | null;
    },
  ) =>
    request<BacklinkView>(`/v1/organizations/${org}/backlinks/${id}`, {
      method: "PATCH",
      headers: csrfHeader(),
      body: JSON.stringify(body),
    }),
  // Bulk monitoring change over a set of backlink ids (§15). Any subset of
  // enabled/level/frequency; omitted fields are left unchanged.
  bulkMonitoring: (
    org: string,
    ids: string[],
    body: { monitoring_enabled?: boolean; monitoring_level?: string; check_frequency?: string },
  ) =>
    request<{ updated: number }>(`/v1/organizations/${org}/backlinks/bulk-monitoring`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify({ ids, ...body }),
    }),
  importCsv: (org: string, projectId: string, csv: string) =>
    request<ImportResult>(`/v1/organizations/${org}/imports`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify({ project_id: projectId, csv }),
    }),
  tags: (org: string) => request<TagView[]>(`/v1/organizations/${org}/tags`),
  createTag: (org: string, name: string, color?: string) =>
    request<TagView>(`/v1/organizations/${org}/tags`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify({ name, color }),
    }),
  backlinkTags: (org: string, id: string) =>
    request<TagView[]>(`/v1/organizations/${org}/backlinks/${id}/tags`),
  backlinkTagsBulk: (org: string, ids: string[]) =>
    request<Record<string, TagView[]>>(
      `/v1/organizations/${org}/backlink-tags?ids=${encodeURIComponent(ids.join(","))}`,
    ),
  attachTag: (org: string, id: string, tagId: string) =>
    request<null>(`/v1/organizations/${org}/backlinks/${id}/tags`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify({ tag_id: tagId }),
    }),
  detachTag: (org: string, id: string, tagId: string) =>
    request<null>(`/v1/organizations/${org}/backlinks/${id}/tags/${tagId}`, {
      method: "DELETE",
      headers: csrfHeader(),
    }),
  notificationPreferences: (org: string) =>
    request<PreferenceItem[]>(`/v1/organizations/${org}/notification-preferences`),
  setNotificationPreferences: (org: string, preferences: PreferenceItem[]) =>
    request<PreferenceItem[]>(`/v1/organizations/${org}/notification-preferences`, {
      method: "PUT",
      headers: csrfHeader(),
      body: JSON.stringify({ preferences }),
    }),
  alertDestinations: (org: string) =>
    request<AlertDestination[]>(`/v1/organizations/${org}/alert-destinations`),
  createAlertDestination: (
    org: string,
    kind: string,
    target: string,
    name?: string,
    digest?: string,
  ) =>
    request<AlertDestination>(`/v1/organizations/${org}/alert-destinations`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify({ kind, target, name, digest }),
    }),
  toggleAlertDestination: (org: string, id: string, enabled: boolean) =>
    request<AlertDestination>(`/v1/organizations/${org}/alert-destinations/${id}`, {
      method: "PATCH",
      headers: csrfHeader(),
      body: JSON.stringify({ enabled }),
    }),
  setAlertDigest: (org: string, id: string, digest: string) =>
    request<AlertDestination>(`/v1/organizations/${org}/alert-destinations/${id}`, {
      method: "PATCH",
      headers: csrfHeader(),
      body: JSON.stringify({ digest }),
    }),
  deleteAlertDestination: (org: string, id: string) =>
    request<null>(`/v1/organizations/${org}/alert-destinations/${id}`, {
      method: "DELETE",
      headers: csrfHeader(),
    }),
  testAlertDestination: (org: string, id: string) =>
    request<{ ok: boolean; http_status: number | null; detail: string }>(
      `/v1/organizations/${org}/alert-destinations/${id}/test`,
      { method: "POST", headers: csrfHeader() },
    ),
  billingStatus: (org: string) =>
    request<{
      plan: string;
      status: string;
      configured: boolean;
      trial_ends_at?: string;
      trial_days_left?: number;
      access_active: boolean;
    }>(`/v1/organizations/${org}/billing`),
  billingUsage: (org: string) =>
    request<UsageView>(`/v1/organizations/${org}/billing/usage`),
  startCheckout: (org: string, plan: string, interval: "monthly" | "annual" = "monthly") =>
    request<{ url: string }>(`/v1/organizations/${org}/billing/checkout`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify({ plan, interval }),
    }),
  openBillingPortal: (org: string) =>
    request<{ url: string }>(`/v1/organizations/${org}/billing/portal`, {
      method: "POST",
      headers: csrfHeader(),
    }),
  clients: (org: string) => request<ClientView[]>(`/v1/organizations/${org}/clients`),
  createClient: (org: string, body: NewClient) =>
    request<ClientView>(`/v1/organizations/${org}/clients`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify(body),
    }),
  updateClient: (org: string, id: string, body: NewClient) =>
    request<ClientView>(`/v1/organizations/${org}/clients/${id}`, {
      method: "PATCH",
      headers: csrfHeader(),
      body: JSON.stringify(body),
    }),
  deleteClient: (org: string, id: string) =>
    request<null>(`/v1/organizations/${org}/clients/${id}`, {
      method: "DELETE",
      headers: csrfHeader(),
    }),
  members: (org: string) => request<MemberView[]>(`/v1/organizations/${org}/members`),
  invitations: (org: string) => request<InvitationView[]>(`/v1/organizations/${org}/invitations`),
  createInvitation: (org: string, email: string, role: string) =>
    request<InvitationView>(`/v1/organizations/${org}/invitations`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify({ email, role }),
    }),
  revokeInvitation: (org: string, id: string) =>
    request<null>(`/v1/organizations/${org}/invitations/${id}`, {
      method: "DELETE",
      headers: csrfHeader(),
    }),
};

export interface ImportRowError {
  row_number: number;
  message: string;
}
export interface ImportResult {
  total: number;
  imported: number;
  duplicates: number;
  errors: ImportRowError[];
}

export interface EventView {
  event_type: string;
  previous_data: unknown;
  current_data: unknown;
  detected_at: string;
}

// ─── Platform admin backoffice (only usable by platform admins; 404 otherwise) ──
export interface AdminStats {
  users: number;
  organizations: number;
  projects: number;
  backlinks: number;
  backlinks_live: number;
  backlinks_lost: number;
}
export interface AdminOrg {
  id: string;
  name: string;
  slug: string;
  created_at: string;
  suspended: boolean;
  plan: string;
  members: number;
  projects: number;
  backlinks: number;
  owner_email: string | null;
}
export interface AdminAudit {
  id: string;
  created_at: string;
  organization_id: string | null;
  org_name: string | null;
  actor_email: string | null;
  action: string;
  resource_type: string | null;
  resource_id: string | null;
  after: string | null;
}
export const ADMIN_PLANS = ["starter", "pro", "business", "agency"] as const;
export interface AdminUser {
  id: string;
  email: string;
  name: string;
  is_platform_admin: boolean;
  email_verified: boolean;
  created_at: string;
  organizations: number;
}

// Founder-ops read models (docs/19) — real data from existing tables.
export interface AdminOps {
  window_secs: number;
  queue: {
    pending: number;
    processing: number;
    failed: number;
    dead: number;
    oldest_pending_at: string | null;
    last_completed_at: string | null;
    created_last_hour: number;
  };
  monitoring: { checks: number; failed: number; avg_duration_ms: number | null; lost_detected: number };
  email: { sent: number; failed: number };
}
export interface AdminBilling {
  mrr_cents: number;
  paying: number;
  trialing: number;
  trialing_active: number;
  trialing_expired: number;
  conversion_pct: number | null;
  past_due: number;
  canceled: number;
  comped: number;
  by_plan: { plan: string; subscribers: number; mrr_cents: number }[];
}
export interface AdminSubscription {
  id: string;
  name: string;
  owner_email: string | null;
  plan: string;
  status: string;
  mrr_cents: number;
  comped: boolean;
  trial_ends_at: string | null;
  current_period_end: string | null;
  cancel_at_period_end: boolean;
  canceled_at: string | null;
}
export interface AdminUsageLimit {
  id: string;
  name: string;
  plan: string;
  backlinks: number;
  limit: number | null;
  pct: number | null;
}
export interface AdminAnalytics {
  window_secs: number;
  signups: number;
  trials_started: number;
  activated: number;
  subscribed: number;
  by_source: { source: string; signups: number }[];
}

export interface AdminWaitlist {
  total: number;
  pending: number;
  recent: { email: string; created_at: string; source: string | null }[];
}

export interface AdminFeedback {
  open: number;
  items: FeedbackItem[];
}

export const adminApi = {
  stats: () => request<AdminStats>("/v1/admin/stats"),
  feedback: () => request<AdminFeedback>("/v1/admin/feedback"),
  setFeedbackStatus: (id: string, status: string) =>
    request<{ ok: boolean; status: string }>(`/v1/admin/feedback/${id}/status`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify({ status }),
    }),
  waitlist: () => request<AdminWaitlist>("/v1/admin/waitlist"),
  resendWaitlistConfirmations: () =>
    request<{ sent: number; failed: number; remaining: number }>("/v1/admin/waitlist/resend", {
      method: "POST",
      headers: csrfHeader(),
    }),
  syncWaitlistAudience: () =>
    request<{ synced: number; failed: number; total: number; audience: string; error: string | null }>(
      "/v1/admin/waitlist/sync-audience",
      {
        method: "POST",
        headers: csrfHeader(),
      },
    ),
  organizations: () => request<AdminOrg[]>("/v1/admin/organizations"),
  users: () => request<AdminUser[]>("/v1/admin/users"),
  audit: () => request<AdminAudit[]>("/v1/admin/audit"),
  ops: (windowHours?: number) =>
    request<AdminOps>(`/v1/admin/ops${windowHours ? `?window_hours=${windowHours}` : ""}`),
  billing: () => request<AdminBilling>("/v1/admin/billing"),
  subscriptions: () => request<AdminSubscription[]>("/v1/admin/subscriptions"),
  analytics: (windowHours?: number) =>
    request<AdminAnalytics>(`/v1/admin/analytics${windowHours ? `?window_hours=${windowHours}` : ""}`),
  usageLimits: () => request<AdminUsageLimit[]>("/v1/admin/usage/limits"),
  setAdmin: (userId: string, isAdmin: boolean) =>
    request<AdminUser>(`/v1/admin/users/${userId}/admin`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify({ is_admin: isAdmin }),
    }),
  setOrgSuspended: (orgId: string, suspended: boolean) =>
    request<null>(`/v1/admin/organizations/${orgId}/suspend`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify({ suspended }),
    }),
  setOrgPlan: (orgId: string, plan: string) =>
    request<null>(`/v1/admin/organizations/${orgId}/plan`, {
      method: "POST",
      headers: csrfHeader(),
      body: JSON.stringify({ plan }),
    }),
  deleteOrg: (orgId: string) =>
    request<null>(`/v1/admin/organizations/${orgId}`, {
      method: "DELETE",
      headers: csrfHeader(),
    }),
};
