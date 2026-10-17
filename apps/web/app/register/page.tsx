"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api } from "../../lib/api";
import { readAttribution } from "../../lib/attribution";
import { Turnstile, TURNSTILE_ENABLED } from "../turnstile";
import { WaitlistModal } from "./WaitlistModal";
import "../auth.css";

export default function RegisterPage() {
  const router = useRouter();
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [token, setToken] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  // null = still checking; true/false = sign-ups open or closed.
  const [open, setOpen] = useState<boolean | null>(null);
  // Waitlist modal shows automatically once we learn sign-ups are closed.
  const [showWaitlist, setShowWaitlist] = useState(false);

  useEffect(() => {
    api.config().then((r) => {
      const isOpen = r.ok ? r.data.registration_open : true;
      setOpen(isOpen);
      if (!isOpen) setShowWaitlist(true);
    });
  }, []);

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
      // The server may have closed sign-ups between page load and submit.
      if (reg.status === 403) setOpen(false);
      setError(reg.detail);
      return;
    }
    // Registration succeeds then logs in for a smooth first run.
    const login = await api.login(email, password);
    setBusy(false);
    if (login.ok) router.push("/dashboard");
    else router.push("/login");
  }

  if (open === false) {
    return (
      <div className="auth-wrap">
        <div className="auth-card">
          <h1>We&apos;re almost ready</h1>
          <p className="sub">
            New sign-ups aren&apos;t open just yet. We&apos;re putting the final touches on LinkPulz.
            Join the waitlist and we&apos;ll email you the moment early access opens.
          </p>
          <button className="auth-btn" type="button" onClick={() => setShowWaitlist(true)}>
            Join the waitlist
          </button>
          <p className="auth-alt" style={{ marginTop: 14 }}>
            Already have an account? <a href="/login">Log in</a>
          </p>
          <p className="auth-alt">
            Want to look around first? <a href="/">See what LinkPulz does</a>
          </p>
        </div>
        {showWaitlist && <WaitlistModal onClose={() => setShowWaitlist(false)} />}
      </div>
    );
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
        <button className="auth-btn" type="submit" disabled={busy || open === null || (TURNSTILE_ENABLED && !token)}>
          {busy ? "Creating…" : "Create account"}
        </button>
        <p className="auth-alt">
          Already have an account? <a href="/login">Log in</a>
        </p>
      </form>
    </div>
  );
}
