import type { Metadata } from "next";

export const dynamic = "force-static"; // public content → prerender + edge cache (pairs with the static marketing CSP in middleware.ts)
import { ProseShell, ProseFooter } from "../../prose-chrome";

export const metadata: Metadata = { title: "Sub-processors" };

export default function SubprocessorsPage() {
  return (
    <ProseShell>
      <h1>Sub-processors</h1>
      <p className="prose-updated">Last updated: 8 October 2026</p>

      <p>
        To run LinkPulz, MeedWise relies on the infrastructure and service providers listed below.
        Each one processes data only on our instructions and under a data-processing agreement with
        appropriate safeguards. We keep this list to the minimum needed to operate the Service.
      </p>

      <table>
        <thead>
          <tr>
            <th>Provider</th>
            <th>Purpose</th>
            <th>Location</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>Vercel Inc.</td>
            <td>Website hosting &amp; content delivery (CDN)</td>
            <td>United States</td>
          </tr>
          <tr>
            <td>Render Services, Inc.</td>
            <td>Application (API &amp; worker) hosting</td>
            <td>United States</td>
          </tr>
          <tr>
            <td>Neon Inc.</td>
            <td>Managed PostgreSQL database hosting</td>
            <td>United States / EU</td>
          </tr>
          <tr>
            <td>Stripe, Inc. / Stripe Payments Europe, Ltd.</td>
            <td>Payment processing &amp; subscription billing (paid plans)</td>
            <td>United States / Ireland</td>
          </tr>
          <tr>
            <td>Resend (Plus Five Five, Inc.)</td>
            <td>Transactional &amp; alert email delivery</td>
            <td>United States</td>
          </tr>
          <tr>
            <td>Cloudflare, Inc.</td>
            <td>Anti-spam / bot protection (Turnstile)</td>
            <td>United States</td>
          </tr>
          <tr>
            <td>Google (Google Ireland Ltd. / Google LLC)</td>
            <td>Consent-based website analytics (GA4) and, if you connect it, Search Console data</td>
            <td>Ireland / United States</td>
          </tr>
        </tbody>
      </table>

      <h2>International transfers</h2>
      <p>
        Where a provider processes data outside the European Economic Area, the transfer is covered by
        appropriate safeguards such as the European Commission&apos;s Standard Contractual Clauses.
      </p>

      <h2>Changes</h2>
      <p>
        We may add or replace sub-processors as the Service evolves. Material changes will be reflected
        by the &quot;Last updated&quot; date above. Questions:{" "}
        <a href="mailto:contact@linkpulz.com">contact@linkpulz.com</a>.
      </p>

      <ProseFooter />
    </ProseShell>
  );
}
