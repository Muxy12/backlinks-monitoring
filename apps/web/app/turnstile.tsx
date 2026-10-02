"use client";

import { useEffect, useRef } from "react";

// Cloudflare Turnstile widget (anti-spam on public forms). Gated on the public
// site key: when it is unset the component renders nothing and forms submit as
// before — the API skips verification unless its secret is also set. The script
// is injected programmatically, so it inherits trust under the app's
// `strict-dynamic` CSP (see middleware.ts, which also opens frame-src for the
// widget's Cloudflare iframe when the key is present).

const SITE_KEY = process.env.NEXT_PUBLIC_TURNSTILE_SITE_KEY;

/** True when Turnstile is configured — forms use it to require a token. */
export const TURNSTILE_ENABLED = !!SITE_KEY;

type RenderOpts = {
  sitekey: string;
  callback: (token: string) => void;
  "expired-callback"?: () => void;
  "error-callback"?: () => void;
  theme?: "auto" | "light" | "dark";
  size?: "normal" | "flexible" | "compact";
};
type TurnstileApi = {
  render: (el: HTMLElement, opts: RenderOpts) => string;
  remove: (id: string) => void;
  reset: (id?: string) => void;
};
declare global {
  interface Window {
    turnstile?: TurnstileApi;
    __turnstileLoading?: Promise<void>;
  }
}

const SCRIPT_SRC = "https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit";

function loadScript(): Promise<void> {
  if (typeof window === "undefined") return Promise.resolve();
  if (window.turnstile) return Promise.resolve();
  if (window.__turnstileLoading) return window.__turnstileLoading;
  window.__turnstileLoading = new Promise<void>((resolve, reject) => {
    const s = document.createElement("script");
    s.src = SCRIPT_SRC;
    s.async = true;
    s.defer = true;
    s.onload = () => resolve();
    s.onerror = () => reject(new Error("Turnstile failed to load"));
    document.head.appendChild(s);
  });
  return window.__turnstileLoading;
}

/**
 * Renders the widget and streams its token to `onToken` (null on expiry/error so
 * the parent can re-disable submit). Renders nothing when Turnstile is off.
 */
export function Turnstile({ onToken }: { onToken: (token: string | null) => void }) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!SITE_KEY) return;
    let cancelled = false;
    let widgetId: string | null = null;

    loadScript()
      .then(() => {
        if (cancelled || !ref.current || !window.turnstile) return;
        widgetId = window.turnstile.render(ref.current, {
          sitekey: SITE_KEY,
          callback: (token) => onToken(token),
          "expired-callback": () => onToken(null),
          "error-callback": () => onToken(null),
          theme: "auto",
          size: "flexible",
        });
      })
      .catch(() => onToken(null));

    return () => {
      cancelled = true;
      if (widgetId && window.turnstile) {
        try {
          window.turnstile.remove(widgetId);
        } catch {
          /* widget already gone */
        }
      }
    };
  }, [onToken]);

  if (!SITE_KEY) return null;
  return <div ref={ref} className="turnstile" style={{ margin: "4px 0 16px" }} />;
}
