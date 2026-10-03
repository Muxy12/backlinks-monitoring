"use client";

import { useState } from "react";
import { api } from "@/lib/api";
import { readAttribution } from "@/lib/attribution";
import { Turnstile, TURNSTILE_ENABLED } from "./turnstile";
import "./waitlist.css";

const LOGO = "https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/logo-linkpulz.webp";
const PREVIEW = "https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/banniere-dashboard-linkpulz.webp";

const FEATURES: { icon: string; title: string; body: string }[] = [
  { icon: "48h", title: "Checks every 48 hours", body: "Every backlink you track is re-checked on a fixed schedule — automatically." },
  { icon: "🔔", title: "Instant change alerts", body: "Know the moment a link is lost, redirected, nofollowed, noindexed or broken." },
  { icon: "🔗", title: "Lost & broken detection", body: "Catch links that quietly disappear or start pointing to error pages." },
  { icon: "📊", title: "Campaigns, costs & reports", body: "Tie links to campaigns and budgets, and prove what you earned or paid for." },
];

export function WaitlistLanding() {
  const [email, setEmail] = useState("");
  const [token, setToken] = useState<string | null>(null);
  const [status, setStatus] = useState<"idle" | "busy" | "done">("idle");
  const [already, setAlready] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (TURNSTILE_ENABLED && !token) {
      setError("Please complete the anti-spam check.");
      return;
    }
    setStatus("busy");
    setError(null);
    const res = await api.joinWaitlist(email.trim(), token ?? undefined, readAttribution());
    if (!res.ok) {
      setStatus("idle");
      setError(res.detail);
      return;
    }
    setAlready(res.data.already);
    setStatus("done");
  }

  return (
    <div className="wl">
      <div className="wl-bg" aria-hidden="true" />

      <header className="wl-top">
        <a className="wl-brand" href="/">
          <img src={LOGO} alt="" width={1254} height={1254} />
          Link<span>Pulz</span>
        </a>
        <a className="wl-login" href="/login">Log in</a>
      </header>

      <main className="wl-main">
        <section className="wl-hero">
          <span className="wl-badge"><span className="wl-dot" />Launching soon</span>
          <h1>Never lose a backlink in silence again.</h1>
          <p className="wl-lead">
            LinkPulz watches every backlink you earn or buy — automatic checks every 48 hours and an
            alert the moment a link is lost, nofollowed, redirected or broken. We're putting the
            finishing touches on it now.
          </p>

          {status === "done" ? (
            <div className="wl-done" role="status">
              <div className="wl-check" aria-hidden="true">✓</div>
              <h2>{already ? "You're already on the list" : "You're on the list!"}</h2>
              <p>We'll email you the moment early access opens. No spam in the meantime — promise.</p>
            </div>
          ) : (
            <>
              <form className="wl-form" onSubmit={submit}>
                <input
                  type="email"
                  inputMode="email"
                  autoComplete="email"
                  placeholder="you@company.com"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  required
                  aria-label="Email address"
                />
                <button type="submit" disabled={status === "busy" || (TURNSTILE_ENABLED && !token)}>
                  {status === "busy" ? "Joining…" : "Join the waitlist"}
                </button>
              </form>
              {TURNSTILE_ENABLED && (
                <div className="wl-turnstile">
                  <Turnstile onToken={setToken} />
                </div>
              )}
              {error && <p className="wl-error">{error}</p>}
              <p className="wl-micro">Early access + one email when we launch. Unsubscribe anytime.</p>
            </>
          )}
        </section>

        <section className="wl-preview" aria-label="A preview of LinkPulz">
          <div className="wl-preview-frame">
            <img src={PREVIEW} alt="Preview of the LinkPulz backlink monitoring dashboard" loading="lazy" />
          </div>
        </section>

        <section className="wl-features" aria-label="What LinkPulz does">
          {FEATURES.map((f) => (
            <div className="wl-card" key={f.title}>
              <div className="wl-ic">{f.icon}</div>
              <h3>{f.title}</h3>
              <p>{f.body}</p>
            </div>
          ))}
        </section>
      </main>

      <footer className="wl-footer">
        <span>© {new Date().getFullYear()} LinkPulz. Backlink monitoring software.</span>
        <nav aria-label="Legal">
          <a href="/legal/privacy">Privacy</a>
          <a href="/legal/terms">Terms</a>
        </nav>
      </footer>
    </div>
  );
}
