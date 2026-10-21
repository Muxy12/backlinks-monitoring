"use client";

import { useState } from "react";
import { api, type BacklinkCheckResult } from "../../../lib/api";
import { Turnstile, TURNSTILE_ENABLED } from "../../turnstile";
import "./checker.css";

const ERROR_LABELS: Record<string, string> = {
  blocked: "That URL can't be checked (it points to a private or disallowed address).",
  response_too_large: "The source page is too large to check.",
  timeout: "The source page timed out.",
  too_many_redirects: "The source page redirected too many times.",
  unreachable: "Couldn't reach the source page.",
  invalid_url: "That source URL isn't valid.",
  error: "Something went wrong fetching the source page.",
};

// The interactive checker form. Extracted into its own client component so the
// route's page.tsx can be a server component (metadata, JSON-LD and the static
// SEO content around the tool). The form posts to the API from the browser.
export function BacklinkCheckerForm() {
  const [sourceUrl, setSourceUrl] = useState("");
  const [targetUrl, setTargetUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const [token, setToken] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<BacklinkCheckResult | null>(null);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (TURNSTILE_ENABLED && !token) {
      setError("Please complete the anti-spam challenge.");
      return;
    }
    setBusy(true);
    setError(null);
    setResult(null);
    const res = await api.checkBacklink(sourceUrl.trim(), targetUrl.trim(), token ?? undefined);
    setBusy(false);
    if (!res.ok) {
      setError(res.detail);
      return;
    }
    setResult(res.data);
  }

  return (
    <div className="checker-card">
      <p className="checker-sub">
        Paste the page that should link to you and your target URL. We&apos;ll fetch the page and
        tell you if the link is live, its anchor text, and whether it&apos;s dofollow.
      </p>

      <form onSubmit={onSubmit}>
        <label className="checker-field">
          <span>Source page URL</span>
          <input
            type="url"
            placeholder="https://a-publisher.com/their-article"
            value={sourceUrl}
            onChange={(e) => setSourceUrl(e.target.value)}
            required
          />
        </label>
        <label className="checker-field">
          <span>Your target URL</span>
          <input
            type="url"
            placeholder="https://your-site.com/page"
            value={targetUrl}
            onChange={(e) => setTargetUrl(e.target.value)}
            required
          />
        </label>
        <Turnstile onToken={setToken} />
        <button className="checker-btn" type="submit" disabled={busy || (TURNSTILE_ENABLED && !token)}>
          {busy ? "Checking…" : "Check backlink"}
        </button>
      </form>

      {error && <div className="checker-error" role="alert">{error}</div>}

      {result && <ResultCard result={result} />}
    </div>
  );
}

function ResultCard({ result }: { result: BacklinkCheckResult }) {
  if (result.error) {
    return (
      <div className="checker-result state-error" role="alert">
        <strong>Couldn&apos;t check</strong>
        <p>{ERROR_LABELS[result.error] ?? ERROR_LABELS.error}</p>
      </div>
    );
  }
  if (!result.found) {
    return (
      <div className="checker-result state-absent" role="status">
        <strong>Not found</strong>
        <p>
          The source page loaded (HTTP {result.http_status}) but does not link to your target URL.
        </p>
      </div>
    );
  }
  return (
    <div className="checker-result state-found" role="status">
      <strong>Backlink is live ✓</strong>
      <dl className="checker-detail">
        <div>
          <dt>Anchor text</dt>
          <dd>{result.anchor_text ?? "-"}</dd>
        </div>
        <div>
          <dt>Link type</dt>
          <dd>{result.is_dofollow ? "dofollow" : "nofollow"}</dd>
        </div>
        <div>
          <dt>rel</dt>
          <dd>{result.rel && result.rel.length ? result.rel.join(", ") : "-"}</dd>
        </div>
        <div>
          <dt>Source indexable</dt>
          <dd>{result.source_noindex ? "no (noindex)" : "yes"}</dd>
        </div>
      </dl>
    </div>
  );
}
