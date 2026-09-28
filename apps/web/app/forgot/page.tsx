"use client";

import { useState } from "react";
import { api } from "../../lib/api";
import "../auth.css";

export default function ForgotPage() {
  const [email, setEmail] = useState("");
  const [busy, setBusy] = useState(false);
  const [sent, setSent] = useState(false);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    await api.forgotPassword(email);
    setBusy(false);
    setSent(true); // always show the same message (never reveal if the email exists)
  }

  return (
    <div className="auth-wrap">
      <form className="auth-card" onSubmit={onSubmit}>
        <h1>Reset password</h1>
        {sent ? (
          <>
            <p className="sub">
              If an account exists for <b>{email}</b>, a reset link is on its way. Check your inbox.
            </p>
            <p className="auth-alt">
              <a href="/login">Back to log in</a>
            </p>
          </>
        ) : (
          <>
            <p className="sub">Enter your email and we’ll send you a reset link.</p>
            <label className="auth-field">
              <span>Email</span>
              <input type="email" value={email} onChange={(e) => setEmail(e.target.value)} required autoComplete="email" />
            </label>
            <button className="auth-btn" type="submit" disabled={busy}>
              {busy ? "Sending…" : "Send reset link"}
            </button>
            <p className="auth-alt">
              <a href="/login">Back to log in</a>
            </p>
          </>
        )}
      </form>
    </div>
  );
}
