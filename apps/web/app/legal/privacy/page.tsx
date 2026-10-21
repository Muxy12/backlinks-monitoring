import type { Metadata } from "next";

export const dynamic = "force-static"; // public content → prerender + edge cache (pairs with the static marketing CSP in middleware.ts)
import { ProseShell, ProseFooter } from "../../prose-chrome";

export const metadata: Metadata = { title: "Privacy Policy" };

export default function PrivacyPage() {
  return (
    <ProseShell>
      <h1>Privacy Policy</h1>
      <p className="prose-updated">Last updated: 21 October 2026</p>

      <p>
        This Privacy Policy explains how LinkPulz (&quot;we&quot;, &quot;us&quot;) collects, uses and
        protects information when you use our backlink-monitoring service (the &quot;Service&quot;). We
        act as the data controller for your account data and as a data processor for the URLs and
        monitoring data you add on behalf of your own clients.
      </p>
      <p>
        The Service is operated by <strong>MeedWise</strong>, a SASU registered in France (RCS Paris
        109 701 466), with its registered office at 59 rue de Ponthieu, Bureau 326, 75008 Paris,
        France. Full company details are in our <a href="/legal/notice">Legal Notice</a>.
      </p>

      <h2>Information we collect</h2>
      <ul>
        <li>
          <strong>Account data</strong>, your name, email address and a securely hashed password.
        </li>
        <li>
          <strong>Workspace content</strong>, the projects, backlinks, competitors, campaigns,
          clients and notes you create, and the URLs you ask us to monitor.
        </li>
        <li>
          <strong>Monitoring results</strong>, HTTP status, anchor text, rel attributes, redirects
          and timestamps we fetch from the public web for the URLs you track.
        </li>
        <li>
          <strong>Technical data</strong>, a session cookie and standard server logs (IP, user
          agent, timestamps) used to operate and secure the Service.
        </li>
      </ul>

      <h2>How we use it</h2>
      <ul>
        <li>To provide, maintain and improve the Service.</li>
        <li>To monitor the backlinks you configure and notify you of changes.</li>
        <li>To secure accounts, prevent abuse and comply with legal obligations.</li>
        <li>To process payments (via our payment processor) where you have a paid plan.</li>
      </ul>
      <p>We do not sell your personal data.</p>

      <h2>Legal bases (GDPR)</h2>
      <p>
        We process personal data to perform our contract with you, to pursue our legitimate
        interests in operating and securing the Service, to comply with legal obligations, and, where
        required, on the basis of your consent (which you may withdraw at any time).
      </p>

      <h2>Sub-processors</h2>
      <p>
        We rely on a small number of providers to run the Service: Vercel (website hosting), Render
        (application hosting), Neon (database hosting), Stripe (payment processing for paid plans),
        Resend (alert and account emails), Cloudflare (anti-spam) and Google (consent-based analytics
        and, if you connect it, Search Console data). Each processes data only on our instructions and
        under appropriate data-protection terms. The current list, with each provider&apos;s role and
        location, is on our <a href="/legal/subprocessors">sub-processors page</a>. Business customers
        can also review our <a href="/legal/dpa">Data Processing Agreement</a>.
      </p>

      <h2>Data retention</h2>
      <p>
        We keep account and workspace data for as long as your account is active. When you delete
        content or close your account, we delete the associated data within a reasonable period,
        except where we must retain it to meet legal or accounting obligations.
      </p>

      <h2>Your rights</h2>
      <p>
        Depending on your location, you may have the right to access, correct, export, restrict or
        delete your personal data, and to object to certain processing. To exercise these rights,
        contact us at the address below. If you are in the EU, you also have the right to lodge a
        complaint with your supervisory authority: in France, the{" "}
        <a href="https://www.cnil.fr" rel="nofollow noopener" target="_blank">
          CNIL
        </a>
        . If you are in the United Kingdom, your supervisory authority is the{" "}
        <a href="https://ico.org.uk" rel="nofollow noopener" target="_blank">
          Information Commissioner&apos;s Office (ICO)
        </a>
        .
      </p>

      <h2>California privacy rights (CCPA/CPRA)</h2>
      <p>
        If you are a California resident, the California Consumer Privacy Act, as amended by the CPRA,
        gives you specific rights over your personal information.
      </p>
      <p>
        <strong>Categories we collect.</strong> In the last 12 months we have collected: identifiers
        (such as your name and email); commercial information (your plan and payment status, handled by
        our payment processor); internet or network activity (a session cookie and standard server
        logs); and the workspace content you choose to add. We collect these from you directly and from
        your use of the Service, for the business purposes described above.
      </p>
      <p>
        <strong>We do not sell or share your personal information</strong> as those terms are defined
        under the CCPA/CPRA: we do not sell personal information, and we do not share it for
        cross-context behavioral advertising. We have not done so in the preceding 12 months, and we do
        not knowingly sell or share the personal information of minors.
      </p>
      <p>
        <strong>Your rights.</strong> You may request to know and access the personal information we
        hold about you, to correct it, and to delete it, and you have the right not to be discriminated
        against for exercising these rights. Because we do not sell or share personal information, there
        is nothing to opt out of, and we do not use sensitive personal information for purposes that
        would trigger a right to limit its use.
      </p>
      <p>
        To exercise these rights, email{" "}
        <a href="mailto:contact@linkpulz.com">contact@linkpulz.com</a>. We will verify your request
        against your account details, and you may use an authorized agent to make a request on your
        behalf.
      </p>

      <h2>Security</h2>
      <p>
        Passwords are hashed, sessions are opaque and stored only as hashes, all traffic is served
        over HTTPS, and outbound fetches are protected against server-side request forgery. No system
        is perfectly secure, but we take reasonable measures appropriate to the risk.
      </p>

      <h2>International transfers</h2>
      <p>
        We are based in the EU and some of our providers are located in the United States and
        elsewhere, so your data may be processed outside your own country. Where we transfer personal
        data internationally, we rely on appropriate safeguards, such as the European Commission&apos;s
        standard contractual clauses (and the UK International Data Transfer Addendum for data from the
        UK), and, for US providers, their certification under the EU-US and UK Extension of the Data
        Privacy Framework where available.
      </p>

      <h2>Changes</h2>
      <p>
        We may update this policy from time to time. Material changes will be reflected by the
        &quot;Last updated&quot; date above.
      </p>

      <h2>Contact</h2>
      <p>
        Questions about this policy or your data? Email{" "}
        <a href="mailto:contact@linkpulz.com">contact@linkpulz.com</a>, or write to us at MeedWise, 59
        rue de Ponthieu, Bureau 326, 75008 Paris, France.
      </p>

      <ProseFooter />
    </ProseShell>
  );
}
