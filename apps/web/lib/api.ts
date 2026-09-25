// Thin client for the Backlio API. Cookies carry the session, so every request
// uses `credentials: "include"`. Base URL is configured per environment.
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

export const api = {
  register: (email: string, password: string, name: string) =>
    request<UserView>("/v1/auth/register", { method: "POST", body: JSON.stringify({ email, password, name }) }),
  login: (email: string, password: string) =>
    request<{ user: UserView }>("/v1/auth/login", { method: "POST", body: JSON.stringify({ email, password }) }),
  logout: () => request<null>("/v1/auth/logout", { method: "POST" }),
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
  checkBacklink: (sourceUrl: string, targetUrl: string) =>
    request<BacklinkCheckResult>("/v1/tools/backlink-check", {
      method: "POST",
      body: JSON.stringify({ source_url: sourceUrl, target_url: targetUrl }),
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

export const orgApi = {
  dashboard: (org: string) => request<DashboardResponse>(`/v1/organizations/${org}/dashboard`),
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
  backlinks: (org: string, projectId: string) =>
    request<BacklinkView[]>(`/v1/organizations/${org}/backlinks?project_id=${projectId}`),
  searchBacklinks: (
    org: string,
    opts: { projectId?: string; clientId?: string; status?: string; q?: string; tagId?: string },
  ) => {
    const p = new URLSearchParams();
    if (opts.projectId) p.set("project_id", opts.projectId);
    if (opts.clientId) p.set("client_id", opts.clientId);
    if (opts.status) p.set("status", opts.status);
    if (opts.q) p.set("q", opts.q);
    if (opts.tagId) p.set("tag_id", opts.tagId);
    const qs = p.toString();
    return request<BacklinkView[]>(
      `/v1/organizations/${org}/backlinks${qs ? `?${qs}` : ""}`,
    );
  },
  backlinkEvents: (org: string, id: string) =>
    request<EventView[]>(`/v1/organizations/${org}/backlinks/${id}/events`),
  backlinkOccurrences: (org: string, id: string) =>
    request<OccurrenceView[]>(`/v1/organizations/${org}/backlinks/${id}/occurrences`),
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
  clients: (org: string) => request<ClientView[]>(`/v1/organizations/${org}/clients`),
  createClient: (org: string, body: NewClient) =>
    request<ClientView>(`/v1/organizations/${org}/clients`, {
      method: "POST",
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
