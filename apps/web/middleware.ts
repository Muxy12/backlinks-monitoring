import { NextRequest, NextResponse } from "next/server";

// Content-Security-Policy with a per-request nonce (docs/11 S8).
//
// The important lock is **script-src**: 'self' + a fresh nonce + 'strict-dynamic'
// and NO 'unsafe-inline'/'unsafe-eval' in production — so even if attacker text
// from a crawled page reached the DOM, an injected <script> could not run. Next
// automatically stamps the nonce onto its own hydration scripts.
//
// **style-src keeps 'unsafe-inline'** on purpose: the app uses React inline
// `style={{}}` props (status colors, tiles, …), which render as inline style
// attributes that no nonce can cover. That is the standard, low-risk concession;
// the script vector is the one that matters for XSS and it is fully locked.

// Origin to allow in connect-src. Absolute NEXT_PUBLIC_API_URL -> its origin;
// a relative value (e.g. "/api" behind the Next proxy) -> "" (same-origin only).
const API_ORIGIN = (() => {
  const v = process.env.NEXT_PUBLIC_API_URL;
  if (v && /^https?:\/\//.test(v)) {
    try {
      return new URL(v).origin;
    } catch {
      /* fall through */
    }
  }
  return "";
})();

// Cloudflare Turnstile (anti-spam widget) needs its own CSP allowances when
// enabled: its iframe (frame-src) and the script/XHR it issues. Added only when
// the public site key is configured, so the policy stays tight otherwise. The
// host in script-src is a fallback for browsers that ignore 'strict-dynamic'.
const TURNSTILE_ORIGIN = process.env.NEXT_PUBLIC_TURNSTILE_SITE_KEY
  ? "https://challenges.cloudflare.com"
  : "";

// Google Analytics (gtag.js). Loading is consent-gated in the client, but the
// CSP must permit the tag host (script-src fallback for browsers that ignore
// 'strict-dynamic') and the collect beacons (connect-src). Harmless until the
// user consents and GA actually loads. img-src already allows https: pixels.
const GA_SCRIPT = "https://www.googletagmanager.com";
const GA_CONNECT =
  "https://www.googletagmanager.com https://www.google-analytics.com https://*.google-analytics.com https://*.analytics.google.com";

export function middleware(request: NextRequest) {
  const dev = process.env.NODE_ENV !== "production";
  const nonce = btoa(crypto.randomUUID());

  // Dev needs eval + websockets for React Refresh / HMR; prod does not.
  const scriptExtra = dev ? " 'unsafe-eval'" : "";
  const connectSrc = ["'self'", API_ORIGIN, TURNSTILE_ORIGIN, GA_CONNECT, ...(dev ? ["ws:", "http://localhost:*"] : [])]
    .filter(Boolean)
    .join(" ");
  const scriptSrc = ["'self'", `'nonce-${nonce}'`, "'strict-dynamic'", TURNSTILE_ORIGIN, GA_SCRIPT]
    .filter(Boolean)
    .join(" ");
  const frameSrc = ["'self'", TURNSTILE_ORIGIN].filter(Boolean).join(" ");

  const csp = [
    `default-src 'self'`,
    `script-src ${scriptSrc}${scriptExtra}`,
    `style-src 'self' 'unsafe-inline'`,
    `img-src 'self' data: https:`,
    `font-src 'self' data:`,
    `connect-src ${connectSrc}`,
    `frame-src ${frameSrc}`,
    `object-src 'none'`,
    `base-uri 'self'`,
    `form-action 'self'`,
    `frame-ancestors 'none'`,
    `upgrade-insecure-requests`,
  ].join("; ");

  // Pass the nonce + CSP to the render on the request, so Next nonces its
  // scripts; also set the CSP on the response the browser enforces.
  const requestHeaders = new Headers(request.headers);
  requestHeaders.set("x-nonce", nonce);
  requestHeaders.set("content-security-policy", csp);

  const response = NextResponse.next({ request: { headers: requestHeaders } });
  response.headers.set("content-security-policy", csp);
  // Pre-launch: keep the entire site out of search indexes. An X-Robots-Tag
  // header is authoritative site-wide and wins over any per-page `index` meta
  // (the most-restrictive directive applies). Remove this line at launch.
  response.headers.set("X-Robots-Tag", "noindex, nofollow");
  return response;
}

export const config = {
  // Run on pages, not on Next's static assets or the image optimizer.
  matcher: [
    {
      source: "/((?!_next/static|_next/image|favicon.ico).*)",
      missing: [
        { type: "header", key: "next-router-prefetch" },
        { type: "header", key: "purpose", value: "prefetch" },
      ],
    },
  ],
};
