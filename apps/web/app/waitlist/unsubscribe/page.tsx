"use client";

import { Suspense, useState } from "react";
import { useSearchParams } from "next/navigation";
import { api } from "@/lib/api";

// Waitlist unsubscribe landing, reached from the email link (?t=<token>). It
// does NOT unsubscribe on load (so an email client's link prefetch can't remove
// someone), the visitor confirms with a button. The whole site is noindex
// during pre-launch, so this utility page is excluded from search too.

const card: React.CSSProperties = {
  maxWidth: 420,
  width: "100%",
  textAlign: "center",
  background: "var(--surface)",
  border: "1px solid var(--border)",
  borderRadius: 16,
  padding: "34px 28px",
  boxShadow: "0 10px 40px color-mix(in srgb, var(--primary) 8%, transparent)",
};
const h: React.CSSProperties = {
  fontSize: 20,
  fontWeight: 750,
  margin: "16px 0 8px",
  color: "var(--text)",
  letterSpacing: "-0.01em",
};
const p: React.CSSProperties = { color: "var(--muted)", lineHeight: 1.55, margin: 0 };

function Inner() {
  const token = useSearchParams().get("t") ?? "";
  const [status, setStatus] = useState<"idle" | "busy" | "done" | "error">("idle");
  const [msg, setMsg] = useState("");

  async function go() {
    if (!token) {
      setStatus("error");
      setMsg("This unsubscribe link looks invalid or incomplete.");
      return;
    }
    setStatus("busy");
    const res = await api.unsubscribeWaitlist(token);
    if (!res.ok) {
      setStatus("error");
      setMsg(res.detail);
      return;
    }
    setStatus("done");
  }

  return (
    <div
      style={{
        minHeight: "100dvh",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: "var(--bg)",
        padding: 24,
        fontFamily: "var(--font-sans, system-ui, sans-serif)",
      }}
    >
      <div style={card}>
        <div style={{ fontWeight: 800, fontSize: 18, letterSpacing: "-0.02em", color: "var(--text)" }}>
          Link<span style={{ color: "var(--primary)" }}>Pulz</span>
        </div>

        {status === "done" ? (
          <>
            <h1 style={h}>You've been removed</h1>
            <p style={p}>
              You won't receive any more emails about the LinkPulz launch. Changed your mind? You
              can rejoin anytime at <a href="/" style={{ color: "var(--primary)" }}>linkpulz.com</a>.
            </p>
          </>
        ) : status === "error" ? (
          <>
            <h1 style={h}>Something went wrong</h1>
            <p style={{ ...p, marginBottom: 16 }}>{msg}</p>
            <a href="/" style={{ color: "var(--primary)" }}>Back to linkpulz.com</a>
          </>
        ) : (
          <>
            <h1 style={h}>Unsubscribe from the waitlist?</h1>
            <p style={{ ...p, marginBottom: 20 }}>
              You'll stop receiving emails about the LinkPulz launch.
            </p>
            <button
              onClick={go}
              disabled={status === "busy"}
              style={{
                font: "inherit",
                fontSize: 15,
                fontWeight: 700,
                color: "#fff",
                background: "var(--primary)",
                border: "none",
                padding: "11px 22px",
                borderRadius: 10,
                cursor: status === "busy" ? "default" : "pointer",
                opacity: status === "busy" ? 0.6 : 1,
              }}
            >
              {status === "busy" ? "…" : "Yes, unsubscribe"}
            </button>
          </>
        )}
      </div>
    </div>
  );
}

export default function UnsubscribePage() {
  return (
    <Suspense>
      <Inner />
    </Suspense>
  );
}
