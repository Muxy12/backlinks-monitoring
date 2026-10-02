"use client";

import { useEffect, useState } from "react";
import "./cookie-banner.css";

const KEY = "backlio_cookies_ack";

// Lightweight cookie notice. LinkPulz sets only strictly-necessary cookies
// (session + CSRF), so this is an informational notice, not a consent gate —
// there is nothing to opt out of. If optional (e.g. analytics) cookies are ever
// added, this becomes a real consent choice. Acknowledgement is remembered in
// localStorage (per-browser), wrapped so a blocked/again-empty store degrades
// gracefully. Renders nothing on the server / first paint to avoid a hydration
// mismatch, then appears only if not yet acknowledged.
export function CookieBanner() {
  const [show, setShow] = useState(false);

  useEffect(() => {
    try {
      if (localStorage.getItem(KEY) !== "1") setShow(true);
    } catch {
      setShow(true);
    }
  }, []);

  if (!show) return null;

  const ack = () => {
    try {
      localStorage.setItem(KEY, "1");
    } catch {
      /* private mode / blocked — just hide for this session */
    }
    setShow(false);
  };

  return (
    <div className="cookie-banner" role="region" aria-label="Cookie notice">
      <p className="cookie-text">
        We use only essential cookies to keep you signed in and to secure the app. No tracking or
        advertising cookies.{" "}
        <a href="/legal/cookies">Learn more</a>.
      </p>
      <button className="cookie-btn" onClick={ack}>
        Got it
      </button>
    </div>
  );
}
