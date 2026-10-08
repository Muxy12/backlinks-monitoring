import type { Metadata } from "next";
import { ProseShell, ProseFooter } from "../../prose-chrome";

export const metadata: Metadata = { title: "Terms of Service" };

export default function TermsPage() {
  return (
    <ProseShell>
      <h1>Terms of Service</h1>
      <p className="prose-updated">Last updated: 8 October 2026</p>

      <p>
        These Terms of Service (&quot;Terms&quot;) govern your access to and use of LinkPulz (the
        &quot;Service&quot;). The Service is operated by <strong>MeedWise</strong>, a SASU with share
        capital of €1,000, registered with the Paris Trade and Companies Register under RCS Paris 109
        701 466, with its registered office at 59 rue de Ponthieu, Bureau 326, 75008 Paris, France
        (&quot;we&quot;, &quot;us&quot;). By creating an account or using the Service, you agree to
        these Terms. If you are using LinkPulz on behalf of an organization, you represent that you
        have authority to bind that organization.
      </p>

      <h2>The Service</h2>
      <p>
        LinkPulz monitors backlinks and related SEO signals for the URLs you configure. We may add,
        change or remove features over time. We aim for high availability but the Service is provided
        without any guarantee of uninterrupted operation.
      </p>

      <h2>Accounts</h2>
      <ul>
        <li>You are responsible for the activity under your account and for keeping credentials safe.</li>
        <li>You must provide accurate information and be at least the age of majority in your region.</li>
        <li>Organizations may invite members with different roles; owners are responsible for their members.</li>
      </ul>

      <h2>Acceptable use</h2>
      <p>You agree not to:</p>
      <ul>
        <li>Monitor URLs you have no lawful basis to monitor, or use the Service to harass or harm others.</li>
        <li>Attempt to overload, probe, reverse-engineer or circumvent the Service or its security.</li>
        <li>Use the Service to violate any applicable law or third-party right.</li>
      </ul>

      <h2>Plans and payment</h2>
      <p>
        Paid plans are billed in advance on a recurring basis through our payment processor. Plan
        limits (such as the number of monitored backlinks) apply to your subscription. Fees are
        non-refundable except where required by law. You can cancel at any time; access continues
        until the end of the current billing period. See our{" "}
        <a href="/legal/refunds">Refund &amp; Cancellation Policy</a> for details.
      </p>

      <h2>Your content</h2>
      <p>
        You retain all rights to the data you add. You grant us a limited licence to host and process
        it solely to provide the Service. You are responsible for the legality of the content and URLs
        you submit.
      </p>

      <h2>Intellectual property</h2>
      <p>
        The Service, including its software and branding, is owned by MeedWise and its licensors. These
        Terms grant you no rights to our trademarks or source code.
      </p>

      <h2>Disclaimer &amp; limitation of liability</h2>
      <p>
        The Service is provided &quot;as is&quot; without warranties of any kind. To the maximum extent
        permitted by law, LinkPulz will not be liable for indirect, incidental or consequential
        damages, and our total liability will not exceed the amount you paid us in the twelve months
        preceding the claim.
      </p>

      <h2>Termination</h2>
      <p>
        You may stop using the Service at any time. We may suspend or terminate access if you breach
        these Terms or to protect the Service. On termination, your right to use the Service ends.
      </p>

      <h2>Changes</h2>
      <p>
        We may update these Terms; material changes will be reflected by the &quot;Last updated&quot;
        date. Continued use after changes take effect constitutes acceptance.
      </p>

      <h2>Governing law &amp; disputes</h2>
      <p>
        These Terms are governed by French law. For business customers, any dispute relating to the
        Service or these Terms will be subject to the exclusive jurisdiction of the competent courts of
        Paris, France. If you are a consumer, mandatory consumer-protection rules of your country of
        residence continue to apply and you may bring proceedings before the competent courts of your
        domicile; you may also use the EU&apos;s{" "}
        <a href="https://ec.europa.eu/consumers/odr" rel="nofollow noopener" target="_blank">
          online dispute resolution
        </a>{" "}
        platform.
      </p>

      <h2>Contact</h2>
      <p>
        Questions about these Terms? Email{" "}
        <a href="mailto:contact@linkpulz.com">contact@linkpulz.com</a>.
      </p>

      <ProseFooter />
    </ProseShell>
  );
}
