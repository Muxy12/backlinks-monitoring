"use client";

import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import { readAttribution } from "@/lib/attribution";
import { Turnstile, TURNSTILE_ENABLED } from "../turnstile";

// Waitlist capture shown as a modal over the (closed) /register page. Reuses the
// same POST /v1/waitlist flow as the standalone waitlist landing. Dependency-free
// and CSP-safe (inline styles only; no inline scripts).

export function WaitlistModal({ onClose }: { onClose: () => void }) {
  const [email, setEmail] = useState("");
  const [token, setToken] = useState<string | null>(null);
  const [status, setStatus] = useState<"idle" | "busy" | "done">("idle");
  const [already, setAlready] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [count, setCount] = useState<number | null>(null);

  useEffect(() => {
    api.waitlistCount().then((r) => {
      if (r.ok) setCount(r.data.count);
    });
  }, []);

  // Close on Escape.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

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
    if (!res.data.already) setCount((c) => (c != null ? c + 1 : c));
    setStatus("done");
  }

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="wl-modal-title"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 2000,
        display: "grid",
        placeItems: "center",
        padding: 16,
        background: "color-mix(in srgb, var(--text) 55%, transparent)",
        backdropFilter: "blur(4px)",
      }}
    >
      <div
        style={{
          position: "relative",
          width: "100%",
          maxWidth: 440,
          background: "var(--surface)",
          border: "1px solid var(--border)",
          borderRadius: "var(--r-2xl, 18px)",
          boxShadow: "var(--shadow-pop)",
          padding: "28px 26px",
        }}
      >
        <button
          type="button"
          aria-label="Close"
          onClick={onClose}
          style={{
            position: "absolute",
            top: 12,
            right: 12,
            width: 32,
            height: 32,
            borderRadius: 8,
            border: "1px solid var(--border)",
            background: "var(--surface-2)",
            color: "var(--muted)",
            fontSize: 18,
            lineHeight: 1,
            cursor: "pointer",
          }}
        >
          ×
        </button>

        {status === "done" ? (
          <div role="status" style={{ textAlign: "center", padding: "6px 2px" }}>
            <div
              aria-hidden="true"
              style={{
                width: 46,
                height: 46,
                margin: "4px auto 14px",
                display: "grid",
                placeItems: "center",
                borderRadius: "50%",
                background: "color-mix(in srgb, var(--live) 16%, transparent)",
                color: "var(--live)",
                fontSize: 24,
              }}
            >
              ✓
            </div>
            <h2 id="wl-modal-title" style={{ fontSize: 19, marginBottom: 6 }}>
              {already ? "You're already on the list" : "You're on the list!"}
            </h2>
            <p style={{ color: "var(--muted)", fontSize: 14, lineHeight: 1.5 }}>
              We&apos;ll email you the moment early access opens. No spam in the meantime, promise.
            </p>
            <button
              type="button"
              className="auth-btn"
              style={{ marginTop: 18 }}
              onClick={onClose}
            >
              Done
            </button>
          </div>
        ) : (
          <>
            <span
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 7,
                fontSize: 12,
                fontWeight: 650,
                color: "var(--primary)",
                background: "var(--primary-weak)",
                padding: "4px 10px",
                borderRadius: 999,
                marginBottom: 12,
              }}
            >
              <span
                aria-hidden="true"
                style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--primary)" }}
              />
              Launching soon
            </span>
            <h2 id="wl-modal-title" style={{ fontSize: 21, marginBottom: 6 }}>
              Join the waitlist
            </h2>
            <p style={{ color: "var(--muted)", fontSize: 14, lineHeight: 1.5, marginBottom: 16 }}>
              Sign-ups aren&apos;t open yet. Leave your email and we&apos;ll let you know the moment
              early access opens.
            </p>

            <form onSubmit={submit} style={{ display: "grid", gap: 10 }}>
              <input
                type="email"
                inputMode="email"
                autoComplete="email"
                placeholder="you@company.com"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                required
                aria-label="Email address"
                style={{
                  width: "100%",
                  padding: "11px 13px",
                  fontSize: 14,
                  color: "var(--text)",
                  background: "var(--surface-2)",
                  border: "1px solid var(--border)",
                  borderRadius: "var(--r-sm, 8px)",
                }}
              />
              {TURNSTILE_ENABLED && <Turnstile onToken={setToken} />}
              {error && <div className="auth-error" role="alert">{error}</div>}
              <button
                type="submit"
                className="auth-btn"
                disabled={status === "busy" || (TURNSTILE_ENABLED && !token)}
              >
                {status === "busy" ? "Joining…" : "Join the waitlist"}
              </button>
            </form>

            <p style={{ color: "var(--muted-2)", fontSize: 12, marginTop: 12, textAlign: "center" }}>
              Early access + one email when we launch. Unsubscribe anytime.
            </p>

            {count != null && count > 0 && (
              <p
                aria-live="polite"
                style={{ color: "var(--muted)", fontSize: 13, marginTop: 10, textAlign: "center" }}
              >
                <b style={{ color: "var(--text)" }}>{count.toLocaleString()}</b>{" "}
                {count === 1 ? "person has" : "people have"} already joined
              </p>
            )}
          </>
        )}
      </div>
    </div>
  );
}
