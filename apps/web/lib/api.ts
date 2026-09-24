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
}
export interface ProjectView {
  id: string;
  name: string;
  domain: string;
  check_frequency_secs: number;
  status: string;
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

export const orgApi = {
  dashboard: (org: string) => request<DashboardResponse>(`/v1/organizations/${org}/dashboard`),
  projects: (org: string) => request<ProjectView[]>(`/v1/organizations/${org}/projects`),
  backlinks: (org: string, projectId: string) =>
    request<BacklinkView[]>(`/v1/organizations/${org}/backlinks?project_id=${projectId}`),
  backlinkEvents: (org: string, id: string) =>
    request<EventView[]>(`/v1/organizations/${org}/backlinks/${id}/events`),
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
};

export interface EventView {
  event_type: string;
  previous_data: unknown;
  current_data: unknown;
  detected_at: string;
}
