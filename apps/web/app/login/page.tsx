"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { api } from "../../lib/api";
import "../auth.css";

export default function LoginPage() {
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    const res = await api.login(email, password);
    setBusy(false);
    if (res.ok) {
      router.push("/dashboard");
    } else {
      setError(res.status === 401 ? "Invalid email or password." : res.detail);
    }
  }

  return (
    <div className="auth-wrap">
      <form className="auth-card" onSubmit={onSubmit}>
        <img
          className="auth-logo"
          src="https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/logo-linkpulz.webp"
          alt="LinkPulz"
          width="44"
          height="44"
        />
        <h1>Log in</h1>
        <p className="sub">Welcome back to LinkPulz.</p>
        {error && <div className="auth-error" role="alert">{error}</div>}
        <label className="auth-field">
          <span>Email</span>
          <input type="email" value={email} onChange={(e) => setEmail(e.target.value)} required autoComplete="email" />
        </label>
        <label className="auth-field">
          <span>Password</span>
          <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} required autoComplete="current-password" />
        </label>
        <button className="auth-btn" type="submit" disabled={busy}>
          {busy ? "Signing in…" : "Log in"}
        </button>
        <p className="auth-alt">
          <a href="/forgot">Forgot password?</a>
        </p>
        <p className="auth-alt">
          No account? <a href="/register">Start monitoring</a>
        </p>
      </form>
    </div>
  );
}
