"use client";

import { useEffect, useState } from "react";
import "./cookie-banner.css";

// Consent key: "all" = essential + analytics (Google Analytics), "essential" =
// strictly-necessary only (session + CSRF). Stored per-browser in localStorage,
// wrapped so a blocked/empty store degrades gracefully. Analytics never loads
// until the visitor chooses "all" (see app/analytics.tsx), which is what keeps
// us GDPR/ePrivacy-correct: no tracking cookies before consent.
export const CONSENT_KEY = "backlio_consent";

export function CookieBanner() {
  const [show, setShow] = useState(false);

  useEffect(() => {
    try {
      if (!localStorage.getItem(CONSENT_KEY)) setShow(true);
    } catch {
      setShow(true);
    }
  }, []);

  if (!show) return null;

  const choose = (value: "all" | "essential") => {
    try {
      localStorage.setItem(CONSENT_KEY, value);
    } catch {
      /* private mode / blocked, just hide for this session */
    }
    if (value === "all") {
      // Tell the analytics loader to start now, without a reload.
      try {
        window.dispatchEvent(new Event("consent-granted"));
      } catch {
        /* no-op */
      }
    }
    setShow(false);
  };

  return (
    <div className="cookie-banner" role="region" aria-label="Cookie choices">
      <p className="cookie-text">
        We use essential cookies to keep you signed in and secure the app. With your consent we also
        use Google Analytics to understand how the site is used, never advertising cookies.{" "}
        <a href="/legal/cookies">Learn more</a>.
      </p>
      <div className="cookie-actions">
        <button className="cookie-btn ghost" onClick={() => choose("essential")}>
          Essential only
        </button>
        <button className="cookie-btn" onClick={() => choose("all")}>
          Accept analytics
        </button>
      </div>
    </div>
  );
}
