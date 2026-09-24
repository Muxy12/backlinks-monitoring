// Static security headers for every web response (docs/11 S8). CSP is set
// per-request in middleware.ts (it needs a nonce); these are constant.
const securityHeaders = [
  { key: "X-Content-Type-Options", value: "nosniff" },
  { key: "X-Frame-Options", value: "DENY" },
  { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
  { key: "Permissions-Policy", value: "camera=(), microphone=(), geolocation=()" },
  { key: "Strict-Transport-Security", value: "max-age=31536000; includeSubDomains" },
];

// Same-origin API proxy (docs/17). When the API is on a different domain than
// the web app (e.g. Vercel front + Render API), a SameSite=Lax session cookie
// would not survive a cross-site fetch. Set API_PROXY_TARGET to the API origin
// and NEXT_PUBLIC_API_URL=/api: the browser then calls /api/* on its own origin
// and Next forwards it server-side, so the session cookie stays first-party.
const apiProxyTarget = process.env.API_PROXY_TARGET;

/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  // ESLint is not configured yet; `next build` type-checks regardless.
  eslint: { ignoreDuringBuilds: true },
  async headers() {
    return [{ source: "/:path*", headers: securityHeaders }];
  },
  async rewrites() {
    if (!apiProxyTarget) return [];
    return [{ source: "/api/:path*", destination: `${apiProxyTarget}/:path*` }];
  },
};
export default nextConfig;
