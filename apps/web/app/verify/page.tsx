"use client";

import { useEffect, useState } from "react";
import { api } from "../../lib/api";
import "../auth.css";

export default function VerifyPage() {
  const [state, setState] = useState<"working" | "ok" | "error">("working");
  const [detail, setDetail] = useState("");

  useEffect(() => {
    const token = new URLSearchParams(window.location.search).get("token") ?? "";
    if (!token) {
      setState("error");
      setDetail("Missing verification token.");
      return;
    }
    api.verifyEmail(token).then((res) => {
      if (res.ok) setState("ok");
      else {
        setState("error");
        setDetail(res.detail);
      }
    });
  }, []);

  return (
    <div className="auth-wrap">
      <div className="auth-card">
        <h1>Email verification</h1>
        {state === "working" && <p className="sub">Verifying…</p>}
        {state === "ok" && (
          <>
            <p className="sub">✓ Your email is verified. Thanks!</p>
            <a className="auth-btn" href="/login" style={{ display: "block", textAlign: "center", textDecoration: "none" }}>
              Continue to log in
            </a>
          </>
        )}
        {state === "error" && (
          <>
            <div className="auth-error">{detail || "Verification failed."}</div>
            <p className="auth-alt">
              The link may have expired. <a href="/login">Log in</a> and request a new one.
            </p>
          </>
        )}
      </div>
    </div>
  );
}
