import type { Metadata } from "next";

export const dynamic = "force-static"; // public content → prerender + edge cache (pairs with the static marketing CSP in middleware.ts)
import { ProseShell, ProseFooter } from "../../prose-chrome";

export const metadata: Metadata = { title: "Cookie Policy" };

export default function CookiesPage() {
  return (
    <ProseShell>
      <h1>Cookie Policy</h1>
      <p className="prose-updated">Last updated: 6 October 2026</p>

      <p>
        LinkPulz uses a deliberately small number of cookies: strictly necessary ones to run the
        Service, and (only if you agree) analytics cookies to understand how the site is used. We
        do not use advertising or cross-site advertising cookies.
      </p>

      <h2>Cookies we set</h2>
      <ul>
        <li>
          <code>session</code>, a strictly necessary, HttpOnly cookie that keeps you signed in. It
          holds an opaque token, not your identity.
        </li>
        <li>
          <code>csrf</code>, a strictly necessary cookie used to protect state-changing requests
          against cross-site request forgery.
        </li>
      </ul>

      <div className="prose-note">
        Both cookies are essential to operate the Service securely, so they are not subject to
        consent. They are set only once you sign in and are cleared when you sign out.
      </div>

      <h2>Analytics cookies (only with your consent)</h2>
      <p>
        If you choose <strong>“Accept analytics”</strong> on our cookie banner, we load Google
        Analytics 4, which sets <code>_ga</code> and <code>_ga_&lt;id&gt;</code> cookies to measure
        aggregate, anonymised usage (pages viewed, rough traffic sources). We enable IP anonymisation.
        These cookies are <strong>never set</strong> until you consent, and if you choose
        “Essential only” they are not loaded at all.
      </p>

      <h2>Managing cookies</h2>
      <p>
        You can clear or block cookies in your browser settings. Blocking the essential cookies above
        will prevent you from signing in and using the authenticated app. To change or withdraw your
        analytics choice, clear this site’s data in your browser (the banner will ask again) or
        decline analytics the next time you are asked.
      </p>

      <h2>Changes</h2>
      <p>
        If we introduce additional cookies, we will update this page and, where required, ask for your
        consent first.
      </p>

      <h2>Contact</h2>
      <p>
        Questions? Email <a href="mailto:contact@linkpulz.com">contact@linkpulz.com</a>.
      </p>

      <ProseFooter />
    </ProseShell>
  );
}
