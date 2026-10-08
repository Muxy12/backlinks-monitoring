import type { Metadata } from "next";
import { ProseShell, ProseFooter } from "../../prose-chrome";

export const metadata: Metadata = { title: "Legal Notice" };

export default function LegalNoticePage() {
  return (
    <ProseShell>
      <h1>Legal Notice</h1>
      <p className="prose-updated">Last updated: 8 October 2026</p>

      <p>
        LinkPulz (the &quot;Service&quot;, available at linkpulz.com) is published by the company
        identified below. This notice is provided under Article 6 of French Law No. 2004-575 of 21
        June 2004 (LCEN).
      </p>

      <h2>Publisher</h2>
      <dl>
        <dt>Company name</dt>
        <dd>MeedWise</dd>
        <dt>Legal form</dt>
        <dd>SASU (société par actions simplifiée unipersonnelle)</dd>
        <dt>Share capital</dt>
        <dd>€1,000</dd>
        <dt>Registered office</dt>
        <dd>59 rue de Ponthieu, Bureau 326, 75008 Paris, France</dd>
        <dt>Registration</dt>
        <dd>RCS Paris 109 701 466</dd>
        <dt>SIRET</dt>
        <dd>109 701 466 00012</dd>
        <dt>Intra-community VAT</dt>
        <dd>FR94109701466</dd>
        <dt>Phone</dt>
        <dd>+33 7 68 01 52 91</dd>
        <dt>Email</dt>
        <dd>
          <a href="mailto:contact@linkpulz.com">contact@linkpulz.com</a>
        </dd>
        <dt>Publication director</dt>
        <dd>The President of MeedWise</dd>
      </dl>

      <h2>Host</h2>
      <p>The website is hosted by:</p>
      <dl>
        <dt>Host</dt>
        <dd>Vercel Inc.</dd>
        <dt>Address</dt>
        <dd>340 S Lemon Ave #4133, Walnut, CA 91789, United States</dd>
        <dt>Website</dt>
        <dd>
          <a href="https://vercel.com" rel="nofollow noopener" target="_blank">
            vercel.com
          </a>
        </dd>
      </dl>
      <p>
        The application and database infrastructure behind the Service is operated by additional
        providers (Render and Neon). The full list of providers that process data on our behalf is in
        our <a href="/legal/subprocessors">sub-processors list</a>.
      </p>

      <h2>Intellectual property</h2>
      <p>
        The Service, its software, content and the LinkPulz brand are the property of MeedWise and its
        licensors. Any reproduction or use without authorisation is prohibited.
      </p>

      <h2>Personal data</h2>
      <p>
        How we handle personal data is described in our{" "}
        <a href="/legal/privacy">Privacy Policy</a>. You may contact us about your data at{" "}
        <a href="mailto:contact@linkpulz.com">contact@linkpulz.com</a>, and you have the right to lodge
        a complaint with the French data-protection authority, the{" "}
        <a href="https://www.cnil.fr" rel="nofollow noopener" target="_blank">
          CNIL
        </a>
        .
      </p>

      <ProseFooter />
    </ProseShell>
  );
}
