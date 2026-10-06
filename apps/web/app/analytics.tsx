"use client";

import { useEffect } from "react";
import { GA_ID } from "@/lib/site";
import { CONSENT_KEY } from "./cookie-banner";

// Google Analytics 4, loaded ONLY after the visitor consents to analytics (the
// cookie banner stores "all") and ONLY in production — so dev/preview traffic
// never pollutes the property, and no tracking cookie is set before consent.
//
// The tag is injected from client code at runtime. Under our strict CSP
// (`script-src 'self' 'nonce-…' 'strict-dynamic'`), a <script> created by the
// already-trusted app bundle inherits trust via 'strict-dynamic', so gtag.js
// runs without needing a nonce on the injected element (the GA host is also
// allowlisted as a fallback for browsers that ignore 'strict-dynamic').

declare global {
  interface Window {
    dataLayer: unknown[];
    gtag?: (...args: unknown[]) => void;
  }
}

export function Analytics() {
  useEffect(() => {
    if (process.env.NODE_ENV !== "production") return;

    let loaded = false;
    const load = () => {
      if (loaded) return;
      try {
        if (localStorage.getItem(CONSENT_KEY) !== "all") return;
      } catch {
        return;
      }
      loaded = true;

      window.dataLayer = window.dataLayer || [];
      const gtag = (...args: unknown[]) => {
        window.dataLayer.push(args);
      };
      window.gtag = gtag;
      gtag("js", new Date());
      // anonymize_ip trims the last IP octet before storage (extra privacy).
      gtag("config", GA_ID, { anonymize_ip: true });

      const s = document.createElement("script");
      s.async = true;
      s.src = `https://www.googletagmanager.com/gtag/js?id=${GA_ID}`;
      document.head.appendChild(s);
    };

    load(); // already-consented returning visitors
    window.addEventListener("consent-granted", load); // consent given just now
    return () => window.removeEventListener("consent-granted", load);
  }, []);

  return null;
}
