"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api, orgApi, type MeResponse, type PreferenceItem } from "@/lib/api";
import "@/app/auth.css";
import "../../dashboard/dashboard.css";

// The events worth surfacing, and the channels we let users toggle here.
const EVENTS: { key: string; label: string }[] = [
  { key: "backlink_lost", label: "Backlink lost" },
  { key: "backlink_found", label: "Backlink found" },
  { key: "backlink_restored", label: "Backlink restored" },
  { key: "anchor_changed", label: "Anchor changed" },
  { key: "target_changed", label: "Target changed" },
  { key: "dofollow_to_nofollow", label: "Became nofollow" },
  { key: "rel_changed", label: "Rel changed" },
  { key: "source_error", label: "Source error" },
  { key: "target_error", label: "Target error" },
  { key: "noindex_detected", label: "Noindex detected" },
];
const CHANNELS = ["in_app", "email"] as const;

function keyFor(event: string, channel: string) {
  return `${event}:${channel}`;
}

export default function NotificationSettingsPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  // enabled map; a missing key means the default (enabled).
  const [on, setOn] = useState<Record<string, boolean>>({});
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.me().then((res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      setMe(res.data);
      setOrgId(res.data.organizations[0]?.id ?? null);
      setLoading(false);
    });
  }, [router]);

  const load = useCallback(async (org: string) => {
    const res = await orgApi.notificationPreferences(org);
    if (res.ok) {
      const map: Record<string, boolean> = {};
      for (const p of res.data) map[keyFor(p.event_type, p.channel)] = p.enabled;
      setOn(map);
    }
  }, []);

  useEffect(() => {
    if (orgId) void load(orgId);
  }, [orgId, load]);

  const isOn = (event: string, channel: string) => on[keyFor(event, channel)] ?? true;

  function toggle(event: string, channel: string) {
    setSaved(false);
    setOn((m) => ({ ...m, [keyFor(event, channel)]: !isOn(event, channel) }));
  }

  async function save() {
    if (!orgId) return;
    setSaving(true);
    const preferences: PreferenceItem[] = [];
    for (const e of EVENTS) {
      for (const c of CHANNELS) {
        preferences.push({ event_type: e.key, channel: c, enabled: isOn(e.key, c) });
      }
    }
    const res = await orgApi.setNotificationPreferences(orgId, preferences);
    setSaving(false);
    if (res.ok) {
      const map: Record<string, boolean> = {};
      for (const p of res.data) map[keyFor(p.event_type, p.channel)] = p.enabled;
      setOn(map);
      setSaved(true);
    }
  }

  if (loading) return <div className="auth-wrap"><p style={{ color: "var(--muted)" }}>Loading…</p></div>;
  if (!me) return null;

  return (
    <main className="dash">
      <header className="page-head">
        <h1>Notification settings</h1>
        <p className="muted">Choose which changes notify you, and where.</p>
      </header>

      {!orgId ? (
        <p className="muted">Create a workspace first.</p>
      ) : (
        <section className="panel">
          <table className="bl-table">
            <thead>
              <tr>
                <th>Event</th>
                <th style={{ textAlign: "center" }}>In-app</th>
                <th style={{ textAlign: "center" }}>Email</th>
              </tr>
            </thead>
            <tbody>
              {EVENTS.map((e) => (
                <tr key={e.key}>
                  <td>{e.label}</td>
                  {CHANNELS.map((c) => (
                    <td key={c} style={{ textAlign: "center" }}>
                      <input
                        type="checkbox"
                        checked={isOn(e.key, c)}
                        onChange={() => toggle(e.key, c)}
                      />
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
          <div style={{ marginTop: 16, display: "flex", alignItems: "center", gap: 12 }}>
            <button className="auth-btn" style={{ width: "auto", padding: "10px 20px" }} onClick={() => void save()} disabled={saving}>
              {saving ? "Saving…" : "Save preferences"}
            </button>
            {saved && <span className="muted">Saved ✓</span>}
          </div>
          <p className="muted" style={{ marginTop: 10, fontSize: 13 }}>
            Email delivery is being wired up; in-app notifications appear in the bell menu.
          </p>
        </section>
      )}
    </main>
  );
}
