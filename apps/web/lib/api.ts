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
export interface MeResponse {
  user: UserView;
  organizations: OrgView[];
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
    const qs = p.toString();
    return request<BacklinkView[]>(
      `/v1/organizations/${org}/backlinks${qs ? `?${qs}` : ""}`,
    );
  },
  backlinkEvents: (org: string, id: string) =>
    request<EventView[]>(`/v1/organizations/${org}/backlinks/${id}/events`),
  backlinkOccurrences: (org: string, id: string) =>
    request<OccurrenceView[]>(`/v1/organizations/${org}/backlinks/${id}/occurrences`),
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

export const adminApi = {
  stats: () => request<AdminStats>("/v1/admin/stats"),
  waitlist: () => request<AdminWaitlist>("/v1/admin/waitlist"),
  resendWaitlistConfirmations: () =>
    request<{ sent: number; failed: number; remaining: number }>("/v1/admin/waitlist/resend", {
      method: "POST",
      headers: csrfHeader(),
    }),
  syncWaitlistAudience: () =>
    request<{ synced: number; failed: number; total: number }>("/v1/admin/waitlist/sync-audience", {
      method: "POST",
      headers: csrfHeader(),
    }),
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
