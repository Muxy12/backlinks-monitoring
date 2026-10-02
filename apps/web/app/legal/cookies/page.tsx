import type { Metadata } from "next";
import { ProseShell, ProseFooter } from "../../prose-chrome";

export const metadata: Metadata = { title: "Cookie Policy" };

export default function CookiesPage() {
  return (
    <ProseShell>
      <h1>Cookie Policy</h1>
      <p className="prose-updated">Last updated: 26 September 2026</p>

      <p>
        LinkPulz uses a deliberately small number of cookies. We do not use advertising or
        cross-site tracking cookies.
      </p>

      <h2>Cookies we set</h2>
      <ul>
        <li>
          <code>session</code> — a strictly necessary, HttpOnly cookie that keeps you signed in. It
          holds an opaque token, not your identity.
        </li>
        <li>
          <code>csrf</code> — a strictly necessary cookie used to protect state-changing requests
          against cross-site request forgery.
        </li>
      </ul>

      <div className="prose-note">
        Both cookies are essential to operate the Service securely, so they are not subject to
        consent. They are set only once you sign in and are cleared when you sign out.
      </div>

      <h2>Managing cookies</h2>
      <p>
        You can clear or block cookies in your browser settings. Blocking the essential cookies above
        will prevent you from signing in and using the authenticated app.
      </p>

      <h2>Changes</h2>
      <p>
        If we introduce additional cookies (for example, optional analytics), we will update this page
        and, where required, ask for your consent first.
      </p>

      <h2>Contact</h2>
      <p>
        Questions? Email <a href="mailto:privacy@linkpulz.com">privacy@linkpulz.com</a>.
      </p>

      <ProseFooter />
    </ProseShell>
  );
}
