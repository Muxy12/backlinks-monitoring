"use client";

import { useEffect, useState } from "react";
import { api } from "../../lib/api";
import "../auth.css";

export default function ResetPage() {
  const [token, setToken] = useState("");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);

  useEffect(() => {
    setToken(new URLSearchParams(window.location.search).get("token") ?? "");
  }, []);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    if (!token) {
      setError("Missing or invalid reset link.");
      return;
    }
    if (password.length < 8) {
      setError("Password must be at least 8 characters.");
      return;
    }
    if (password !== confirm) {
      setError("Passwords don’t match.");
      return;
    }
    setBusy(true);
    const res = await api.resetPassword(token, password);
    setBusy(false);
    if (res.ok) setDone(true);
    else setError(res.detail);
  }

  return (
    <div className="auth-wrap">
      <form className="auth-card" onSubmit={onSubmit}>
        <h1>Set a new password</h1>
        {done ? (
          <>
            <p className="sub">✓ Your password has been reset. You can log in now.</p>
            <a className="auth-btn" href="/login" style={{ display: "block", textAlign: "center", textDecoration: "none" }}>
              Log in
            </a>
          </>
        ) : (
          <>
            {error && <div className="auth-error">{error}</div>}
            <label className="auth-field">
              <span>New password</span>
              <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} required autoComplete="new-password" />
            </label>
            <label className="auth-field">
              <span>Confirm password</span>
              <input type="password" value={confirm} onChange={(e) => setConfirm(e.target.value)} required autoComplete="new-password" />
            </label>
            <button className="auth-btn" type="submit" disabled={busy}>
              {busy ? "Saving…" : "Reset password"}
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
