"use client";

import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { api } from "@/lib/api";
import { readAttribution } from "@/lib/attribution";
import { Turnstile, TURNSTILE_ENABLED } from "./turnstile";

// Newsletter / email capture for the marketing footer. The footer is static HTML
// (injected server-side), so this client component renders its form into the
// `#lp-newsletter` slot via a portal once hydrated — progressive enhancement.
// Emails go through the existing POST /v1/waitlist flow (Turnstile-protected),
// which adds the contact to the Resend audience the newsletter is sent from.
export function NewsletterSignup() {
  const [slot, setSlot] = useState<HTMLElement | null>(null);
  const [email, setEmail] = useState("");
  const [token, setToken] = useState<string | null>(null);
  const [status, setStatus] = useState<"idle" | "busy" | "done">("idle");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setSlot(document.getElementById("lp-newsletter"));
  }, []);

  if (!slot) return null;

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
    setStatus("done");
  }

  const content =
    status === "done" ? (
      <p className="nl-done" role="status">
        ✓ You&apos;re subscribed — watch your inbox for tips and launch news.
      </p>
    ) : (
      <form className="nl-form" onSubmit={submit}>
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
          {status === "busy" ? "Subscribing…" : "Subscribe"}
        </button>
        {TURNSTILE_ENABLED && (
          <div className="nl-ts">
            <Turnstile onToken={setToken} />
          </div>
        )}
        {error && <p className="nl-error">{error}</p>}
      </form>
    );

  return createPortal(content, slot);
}
