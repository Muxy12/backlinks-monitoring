// First-touch marketing attribution (docs/20 §P8). The value is captured
// site-wide by the inline script in app/layout.tsx; this reader hands it to the
// register call. Reading is best-effort — a missing or malformed value is fine.
export interface Attribution {
  utm_source?: string;
  utm_medium?: string;
  utm_campaign?: string;
  utm_term?: string;
  utm_content?: string;
  referrer?: string;
  landing_path?: string;
}

const KEY = "backlio_attribution";

export function readAttribution(): Attribution | undefined {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return undefined;
    const a = JSON.parse(raw) as Attribution;
    return a && typeof a === "object" ? a : undefined;
  } catch {
    return undefined;
  }
}
