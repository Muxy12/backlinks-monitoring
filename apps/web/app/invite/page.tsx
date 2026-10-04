"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api } from "../../lib/api";
import "../auth.css";

type Phase = "loading" | "need-login" | "accepting" | "done" | "error";

export default function InvitePage() {
  const router = useRouter();
  const [phase, setPhase] = useState<Phase>("loading");
  const [message, setMessage] = useState("");

  useEffect(() => {
    // Read the token from the URL on the client (no useSearchParams, so no
    // Suspense boundary is needed at build time).
    const token = new URLSearchParams(window.location.search).get("token") ?? "";
    if (!token) {
      setPhase("error");
      setMessage("This invite link is missing its token.");
      return;
    }

    (async () => {
      const meRes = await api.me();
      if (!meRes.ok) {
        // Keep the token so the user can return after logging in.
        setPhase("need-login");
        return;
      }
      setPhase("accepting");
      const res = await api.acceptInvitation(token);
      if (res.ok) {
        setPhase("done");
        setTimeout(() => router.push("/dashboard"), 1200);
      } else {
        setPhase("error");
        setMessage(
          res.status === 404
            ? "This invite is invalid, expired, already used, or was sent to a different email than the one you're signed in with."
            : res.detail,
        );
      }
    })();
  }, [router]);

  return (
    <div className="auth-wrap">
      <div className="auth-card">
        <h1>Join workspace</h1>
        {phase === "loading" && <p className="sub">Checking your invite…</p>}
        {phase === "accepting" && <p className="sub">Accepting…</p>}
        {phase === "done" && <p className="sub">You&apos;re in, taking you to the dashboard.</p>}
        {phase === "need-login" && (
          <>
            <p className="sub">Log in (or create an account) with the email your invite was sent to, then reopen this link.</p>
            <a className="auth-btn" href="/login" style={{ display: "inline-block", textDecoration: "none", textAlign: "center" }}>
              Log in
            </a>
          </>
        )}
        {phase === "error" && <div className="auth-error">{message}</div>}
      </div>
    </div>
  );
}
