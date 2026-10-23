"use client";
import { PageLoading } from "@/components/PageLoading";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api, type FeedbackItem, type MeResponse } from "@/lib/api";
import { selectedOrgId } from "@/lib/workspace";
import "@/app/auth.css";
import "../dashboard/dashboard.css";

type Kind = "feature" | "bug";

// How each triage status reads to the user, and its badge colors.
const STATUS_META: Record<string, { label: string; bg: string; fg: string }> = {
  open: { label: "Received", bg: "#e5edff", fg: "#1e40af" },
  planned: { label: "Planned", bg: "#ede9fe", fg: "#5b21b6" },
  in_progress: { label: "In progress", bg: "#fef3c7", fg: "#92400e" },
  done: { label: "Shipped", bg: "#dcfce7", fg: "#166534" },
  declined: { label: "Not planned", bg: "#f1f5f9", fg: "#475569" },
};

function statusMeta(status: string) {
  return STATUS_META[status] ?? { label: status, bg: "#f1f5f9", fg: "#475569" };
}

export default function FeedbackPage() {
  const router = useRouter();
  const [, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const [kind, setKind] = useState<Kind>("feature");
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null);
  const [mine, setMine] = useState<FeedbackItem[]>([]);

  const loadMine = useCallback(async () => {
    const r = await api.myFeedback();
    if (r.ok) setMine(r.data);
  }, []);

  useEffect(() => {
    api.me().then((res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      setMe(res.data);
      setOrgId(selectedOrgId(res.data.organizations));
      setLoading(false);
      void loadMine();
    });
  }, [router, loadMine]);

  const submit = async () => {
    const t = title.trim();
    const b = body.trim();
    if (!t || !b) {
      setMsg({ ok: false, text: "Please add a short title and a description." });
      return;
    }
    setBusy(true);
    setMsg(null);
    const res = await api.submitFeedback({
      kind,
      title: t,
      body: b,
      page_url: typeof window !== "undefined" ? window.location.href : null,
      organization_id: orgId,
    });
    setBusy(false);
    if (!res.ok) {
      setMsg({ ok: false, text: res.detail });
      return;
    }
    setTitle("");
    setBody("");
    setMsg({
      ok: true,
      text: kind === "bug" ? "Thanks, your bug report was sent." : "Thanks, your idea was sent.",
    });
    void loadMine();
  };

  if (loading) return <PageLoading />;

  return (
    <main className="dash">
      <header className="page-head">
        <h1>Feedback</h1>
        <p className="muted">
          Suggest a feature or report a bug. Everything goes straight to the team, and you can
          follow the status of your own submissions below.
        </p>
      </header>

      <section className="panel" style={{ maxWidth: 680 }}>
        <div className="seg" role="tablist" aria-label="Feedback type" style={{ display: "flex", gap: 8, marginBottom: 16 }}>
          <button
            type="button"
            role="tab"
            aria-selected={kind === "feature"}
            className={`btn${kind === "feature" ? " primary" : ""}`}
            onClick={() => setKind("feature")}
          >
            Feature idea
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={kind === "bug"}
            className={`btn${kind === "bug" ? " primary" : ""}`}
            onClick={() => setKind("bug")}
          >
            Bug report
          </button>
        </div>

        <label className="auth-field" style={{ margin: "0 0 12px" }}>
          <span>{kind === "bug" ? "What went wrong?" : "Your idea, in a line"}</span>
          <input
            value={title}
            maxLength={160}
            onChange={(e) => setTitle(e.target.value)}
            placeholder={
              kind === "bug" ? "e.g. Export button does nothing on the Projects page" : "e.g. Weekly email digest of lost backlinks"
            }
          />
        </label>

        <label className="auth-field" style={{ margin: "0 0 14px" }}>
          <span>
            {kind === "bug" ? "Steps to reproduce, and what you expected" : "Why would this help? Any details"}
          </span>
          <textarea
            value={body}
            maxLength={4000}
            rows={6}
            onChange={(e) => setBody(e.target.value)}
            placeholder={
              kind === "bug"
                ? "1. Go to…\n2. Click…\n3. I expected… but instead…"
                : "Describe the problem it solves or how you'd use it."
            }
            style={{ resize: "vertical", fontFamily: "inherit" }}
          />
        </label>

        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <button className="btn primary" disabled={busy} onClick={() => void submit()}>
            {busy ? "Sending…" : "Send feedback"}
          </button>
          {msg && (
            <span
              className="sm"
              style={{ color: msg.ok ? "var(--live, #166534)" : "var(--lost, #b91c1c)" }}
            >
              {msg.text}
            </span>
          )}
        </div>
      </section>

      <section className="panel" style={{ marginTop: 18 }}>
        <div className="panel-head">
          <h2>Your submissions</h2>
        </div>
        {mine.length === 0 ? (
          <p className="muted" style={{ marginTop: 6 }}>
            Nothing yet. Your feature ideas and bug reports will appear here once you send them.
          </p>
        ) : (
          <table className="bl-table">
            <thead>
              <tr>
                <th>Type</th>
                <th>Title</th>
                <th>Status</th>
                <th>Sent</th>
              </tr>
            </thead>
            <tbody>
              {mine.map((f) => {
                const sm = statusMeta(f.status);
                return (
                  <tr key={f.id}>
                    <td className="sm">{f.kind === "bug" ? "Bug" : "Feature"}</td>
                    <td title={f.body}>{f.title}</td>
                    <td>
                      <span className="badge" style={{ background: sm.bg, color: sm.fg }}>
                        {sm.label}
                      </span>
                    </td>
                    <td className="sm">{new Date(f.created_at).toLocaleDateString()}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
