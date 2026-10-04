import type { Metadata } from "next";
import { ProseShell, ProseFooter } from "../../prose-chrome";

export const metadata: Metadata = { title: "Privacy Policy" };

export default function PrivacyPage() {
  return (
    <ProseShell>
      <h1>Privacy Policy</h1>
      <p className="prose-updated">Last updated: 26 September 2026</p>

      <p>
        This Privacy Policy explains how LinkPulz (&quot;we&quot;, &quot;us&quot;) collects, uses and
        protects information when you use our backlink-monitoring service (the &quot;Service&quot;). We
        act as the data controller for your account data and as a data processor for the URLs and
        monitoring data you add on behalf of your own clients.
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
        We rely on a small number of infrastructure providers to run the Service, application
        hosting, database hosting, and (for paid plans) payment processing. These providers process
        data only on our instructions and under appropriate data-protection terms.
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
        contact us at the address below.
      </p>

      <h2>Security</h2>
      <p>
        Passwords are hashed, sessions are opaque and stored only as hashes, all traffic is served
        over HTTPS, and outbound fetches are protected against server-side request forgery. No system
        is perfectly secure, but we take reasonable measures appropriate to the risk.
      </p>

      <h2>International transfers</h2>
      <p>
        Your data may be processed in countries other than your own. Where it is, we rely on
        appropriate safeguards such as standard contractual clauses.
      </p>

      <h2>Changes</h2>
      <p>
        We may update this policy from time to time. Material changes will be reflected by the
        &quot;Last updated&quot; date above.
      </p>

      <h2>Contact</h2>
      <p>
        Questions about this policy or your data? Email{" "}
        <a href="mailto:privacy@linkpulz.com">privacy@linkpulz.com</a>.
      </p>

      <ProseFooter />
    </ProseShell>
  );
}
