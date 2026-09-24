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

export const api = {
  register: (email: string, password: string, name: string) =>
    request<UserView>("/v1/auth/register", { method: "POST", body: JSON.stringify({ email, password, name }) }),
  login: (email: string, password: string) =>
    request<{ user: UserView }>("/v1/auth/login", { method: "POST", body: JSON.stringify({ email, password }) }),
  logout: () => request<null>("/v1/auth/logout", { method: "POST" }),
  me: () => request<MeResponse>("/v1/me"),
};
