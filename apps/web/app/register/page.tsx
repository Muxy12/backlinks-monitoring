"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { api } from "../../lib/api";
import { readAttribution } from "../../lib/attribution";
import { Turnstile, TURNSTILE_ENABLED } from "../turnstile";
import "../auth.css";

export default function RegisterPage() {
  const router = useRouter();
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [token, setToken] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (TURNSTILE_ENABLED && !token) {
      setError("Please complete the anti-spam challenge.");
      return;
    }
    setBusy(true);
    setError(null);
    const reg = await api.register(email, password, name, token ?? undefined, readAttribution());
    if (!reg.ok) {
      setBusy(false);
      setError(reg.detail);
      return;
    }
    // Registration succeeds then logs in for a smooth first run.
    const login = await api.login(email, password);
    setBusy(false);
    if (login.ok) router.push("/dashboard");
    else router.push("/login");
  }

  return (
    <div className="auth-wrap">
      <form className="auth-card" onSubmit={onSubmit}>
        <h1>Start monitoring</h1>
        <p className="sub">Create your LinkPulz account.</p>
        {error && <div className="auth-error">{error}</div>}
        <label className="auth-field">
          <span>Name</span>
          <input value={name} onChange={(e) => setName(e.target.value)} required autoComplete="name" />
        </label>
        <label className="auth-field">
          <span>Email</span>
          <input type="email" value={email} onChange={(e) => setEmail(e.target.value)} required autoComplete="email" />
        </label>
        <label className="auth-field">
          <span>Password</span>
          <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} required minLength={8} autoComplete="new-password" />
        </label>
        <Turnstile onToken={setToken} />
        <button className="auth-btn" type="submit" disabled={busy || (TURNSTILE_ENABLED && !token)}>
          {busy ? "Creating…" : "Create account"}
        </button>
        <p className="auth-alt">
          Already have an account? <a href="/login">Log in</a>
        </p>
      </form>
    </div>
  );
}
