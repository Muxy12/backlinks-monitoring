// Multi-org workspace selection. Every authenticated page picks its org through
// these helpers instead of hardcoding organizations[0], so a user with several
// workspaces can switch. The choice lives in localStorage (per browser) and
// falls back to the first membership when unset or no longer valid.
import type { OrgView } from "./api";

const KEY = "backlio_org";

export function selectedOrg(orgs: OrgView[]): OrgView | null {
  if (orgs.length === 0) return null;
  let stored: string | null = null;
  try {
    stored = localStorage.getItem(KEY);
  } catch {
    /* private mode / blocked */
  }
  return orgs.find((o) => o.id === stored) ?? orgs[0];
}

export function selectedOrgId(orgs: OrgView[]): string | null {
  return selectedOrg(orgs)?.id ?? null;
}

export function setSelectedOrg(id: string) {
  try {
    localStorage.setItem(KEY, id);
  } catch {
    /* ignore */
  }
}
