import "./prose.css";

// Shared chrome for the standalone legal + docs pages: a slim top bar and a
// footer, wrapping page content in a readable column. Server component, zero JS.
export function ProseShell({ children }: { children: React.ReactNode }) {
  return (
    <div className="prose-shell">
      <header className="prose-top">
        <div className="prose-top-inner">
          <a className="prose-brand" href="/" aria-label="LinkPulz home">
            <img
              className="prose-logo"
              src="https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/Logo_LinkPulz_en_n%C3%A9on_d%C3%A9grad%C3%A9.png"
              alt="LinkPulz"
              width={866}
              height={288}
            />
          </a>
          <nav className="prose-top-links" aria-label="Site">
            <a href="/docs">Docs</a>
            <a href="/tools/backlink-checker">Free checker</a>
            <a href="/login">Sign in</a>
          </nav>
        </div>
      </header>
      <main className="prose-main">{children}</main>
    </div>
  );
}

export function ProseFooter() {
  const year = new Date().getFullYear();
  return (
    <footer className="prose-footer">
      <span>© {year} LinkPulz, MeedWise</span>
      <a href="/legal/privacy">Privacy</a>
      <a href="/legal/terms">Terms</a>
      <a href="/legal/cookies">Cookies</a>
      <a href="/legal/notice">Legal notice</a>
      <a href="/docs">Docs</a>
    </footer>
  );
}
