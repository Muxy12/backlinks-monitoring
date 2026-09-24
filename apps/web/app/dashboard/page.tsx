"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api, type MeResponse } from "../../lib/api";
import "../auth.css";

export default function DashboardPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.me().then((res) => {
      if (res.ok) setMe(res.data);
      else router.push("/login");
      setLoading(false);
    });
  }, [router]);

  async function onLogout() {
    await api.logout();
    router.push("/login");
  }

  if (loading) {
    return (
      <div className="auth-wrap">
        <p style={{ color: "var(--muted)" }}>Loading…</p>
      </div>
    );
  }
  if (!me) return null;

  return (
    <main style={{ maxWidth: "var(--max)", margin: "0 auto", padding: "40px 24px" }}>
      <header style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 32 }}>
        <div>
          <h1 style={{ fontSize: 24 }}>Dashboard</h1>
          <p style={{ color: "var(--muted)", fontSize: 14 }}>Signed in as {me.user.email}</p>
        </div>
        <button className="auth-btn" style={{ width: "auto", padding: "8px 16px" }} onClick={onLogout}>
          Log out
        </button>
      </header>

      <section
        style={{
          background: "var(--surface)",
          border: "1px solid var(--border)",
          borderRadius: "var(--radius)",
          padding: 24,
        }}
      >
        <h2 style={{ fontSize: 16, marginBottom: 16 }}>Your workspaces</h2>
        {me.organizations.length === 0 ? (
          <p style={{ color: "var(--muted)", fontSize: 14 }}>
            No workspaces yet. Your first workspace and project are the start of monitoring.
          </p>
        ) : (
          <ul style={{ listStyle: "none", display: "grid", gap: 10 }}>
            {me.organizations.map((o) => (
              <li
                key={o.id}
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  padding: "12px 14px",
                  background: "var(--surface-2)",
                  border: "1px solid var(--border)",
                  borderRadius: 10,
                }}
              >
                <span>{o.name}</span>
                <span style={{ color: "var(--muted)", fontSize: 13 }}>{o.role}</span>
              </li>
            ))}
          </ul>
        )}
      </section>
    </main>
  );
}
