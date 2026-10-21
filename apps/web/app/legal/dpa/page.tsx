import type { Metadata } from "next";

export const dynamic = "force-static"; // public content → prerender + edge cache (pairs with the static marketing CSP in middleware.ts)
import { ProseShell, ProseFooter } from "../../prose-chrome";

export const metadata: Metadata = { title: "Data Processing Agreement" };

export default function DpaPage() {
  return (
    <ProseShell>
      <h1>Data Processing Agreement</h1>
      <p className="prose-updated">Last updated: 21 October 2026</p>

      <p>
        This Data Processing Agreement (&quot;DPA&quot;) forms part of the{" "}
        <a href="/legal/terms">Terms of Service</a> between you (the &quot;Customer&quot;) and MeedWise
        (&quot;we&quot;, &quot;us&quot;), operator of LinkPulz, and applies where we process personal
        data on your behalf under the GDPR (Regulation (EU) 2016/679) and, for customers in the United
        Kingdom, the UK GDPR.
      </p>

      <div className="prose-note">
        This page is a readable summary of the terms on which we act as your processor. If your
        organisation requires a signed DPA, email{" "}
        <a href="mailto:contact@linkpulz.com">contact@linkpulz.com</a> and we will provide one.
      </div>

      <h2>Roles</h2>
      <p>
        For the workspace content you add (the URLs you monitor and related data), the Customer acts as
        the <strong>data controller</strong> and MeedWise acts as the <strong>data processor</strong>.
        For account and billing data, MeedWise acts as controller, as described in our{" "}
        <a href="/legal/privacy">Privacy Policy</a>.
      </p>

      <h2>Subject matter &amp; duration</h2>
      <p>
        We process personal data to provide the Service for as long as the Customer&apos;s account is
        active, and until deletion or return as set out below.
      </p>

      <h2>Nature &amp; purpose of processing</h2>
      <p>
        Hosting, storing and processing the data the Customer submits, and monitoring the URLs the
        Customer configures, solely to provide and secure the Service and on the Customer&apos;s
        documented instructions (including through the application&apos;s features).
      </p>

      <h2>Categories of data &amp; data subjects</h2>
      <p>
        The data the Customer chooses to add: typically URLs, anchor text, link metadata, campaign and
        client notes, and any contact details the Customer enters. Data subjects are the people the
        Customer chooses to include (for example, contacts at linking sites). The Customer must not use
        the Service for special categories of data.
      </p>

      <h2>Our obligations</h2>
      <ul>
        <li>Process personal data only on the Customer&apos;s documented instructions.</li>
        <li>Ensure persons authorised to process the data are bound by confidentiality.</li>
        <li>Implement appropriate technical and organisational security measures (see below).</li>
        <li>Assist the Customer, taking into account the nature of processing, with data-subject requests and with security, breach-notification and impact-assessment obligations.</li>
        <li>Notify the Customer without undue delay after becoming aware of a personal-data breach.</li>
      </ul>

      <h2>Security</h2>
      <p>
        Passwords are hashed and sessions are stored only as hashes; all traffic is served over HTTPS;
        tenant data is isolated per workspace; outbound fetches are protected against server-side
        request forgery; and access to production systems is restricted. Measures are kept appropriate
        to the risk.
      </p>

      <h2>Sub-processors</h2>
      <p>
        The Customer authorises us to engage the sub-processors listed on our{" "}
        <a href="/legal/subprocessors">sub-processors page</a>. Each is bound by data-protection terms
        consistent with this DPA. We will update that page before adding a new sub-processor so the
        Customer can object on reasonable data-protection grounds.
      </p>

      <h2>International transfers</h2>
      <p>
        Where personal data is transferred outside the European Economic Area or the United Kingdom,
        the transfer relies on appropriate safeguards, such as the European Commission&apos;s Standard
        Contractual Clauses, the UK International Data Transfer Addendum for data from the UK, and, for
        US sub-processors, their certification under the EU-US and UK Extension of the Data Privacy
        Framework where available. Where the Standard Contractual Clauses apply, they are incorporated
        into this DPA by reference.
      </p>

      <h2>Deletion &amp; return</h2>
      <p>
        On termination, or on the Customer&apos;s request, we delete or return the workspace personal
        data within a reasonable period, except where retention is required by law.
      </p>

      <h2>Audit</h2>
      <p>
        We make available the information reasonably necessary to demonstrate compliance with this DPA
        and will contribute to audits mandated by the Customer, subject to reasonable notice,
        confidentiality and the security of other customers.
      </p>

      <h2>Contact</h2>
      <p>
        Questions or a signed-DPA request:{" "}
        <a href="mailto:contact@linkpulz.com">contact@linkpulz.com</a>.
      </p>

      <ProseFooter />
    </ProseShell>
  );
}
