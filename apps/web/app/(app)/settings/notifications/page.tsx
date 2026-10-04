"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api, orgApi, type AlertDestination, type MeResponse, type PreferenceItem } from "@/lib/api";
import { selectedOrgId } from "@/lib/workspace";
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
  // Alert destinations (webhook / Slack).
  const [dests, setDests] = useState<AlertDestination[]>([]);
  const [dkind, setDkind] = useState("slack");
  const [ddigest, setDdigest] = useState("immediate");
  const [dtarget, setDtarget] = useState("");
  const [dname, setDname] = useState("");
  const [dbusy, setDbusy] = useState(false);
  const [dmsg, setDmsg] = useState<string | null>(null);

  useEffect(() => {
    api.me().then((res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      setMe(res.data);
      setOrgId(selectedOrgId(res.data.organizations));
      setLoading(false);
    });
  }, [router]);

  const load = useCallback(async (org: string) => {
    const [res, d] = await Promise.all([
      orgApi.notificationPreferences(org),
      orgApi.alertDestinations(org),
    ]);
    if (res.ok) {
      const map: Record<string, boolean> = {};
      for (const p of res.data) map[keyFor(p.event_type, p.channel)] = p.enabled;
      setOn(map);
    }
    if (d.ok) setDests(d.data);
  }, []);

  useEffect(() => {
    if (orgId) void load(orgId);
  }, [orgId, load]);

  const reloadDests = useCallback(async (org: string) => {
    const d = await orgApi.alertDestinations(org);
    if (d.ok) setDests(d.data);
  }, []);

  async function addDest() {
    if (!orgId) return;
    setDmsg(null);
    if (!dtarget.trim()) {
      setDmsg("Paste the destination URL.");
      return;
    }
    setDbusy(true);
    const res = await orgApi.createAlertDestination(
      orgId,
      dkind,
      dtarget.trim(),
      dname.trim() || undefined,
      ddigest,
    );
    setDbusy(false);
    if (!res.ok) {
      setDmsg(res.detail);
      return;
    }
    setDtarget("");
    setDname("");
    void reloadDests(orgId);
  }

  async function toggleDest(d: AlertDestination) {
    if (!orgId) return;
    const res = await orgApi.toggleAlertDestination(orgId, d.id, !d.enabled);
    if (res.ok) void reloadDests(orgId);
  }

  async function changeDigest(d: AlertDestination, digest: string) {
    if (!orgId || digest === d.digest) return;
    const res = await orgApi.setAlertDigest(orgId, d.id, digest);
    if (res.ok) void reloadDests(orgId);
    else setDmsg(res.detail);
  }

  async function testDest(d: AlertDestination) {
    if (!orgId) return;
    setDbusy(true);
    setDmsg(`Testing ${d.name || d.kind}…`);
    const res = await orgApi.testAlertDestination(orgId, d.id);
    setDbusy(false);
    setDmsg(res.ok ? res.data.detail : res.detail);
    void reloadDests(orgId);
  }

  async function delDest(d: AlertDestination) {
    if (!orgId) return;
    if (!confirm(`Delete alert destination "${d.name || d.target}"?`)) return;
    const res = await orgApi.deleteAlertDestination(orgId, d.id);
    if (res.ok) void reloadDests(orgId);
  }

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
        <>
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

        <section className="panel" style={{ marginTop: 16 }}>
          <div className="panel-head"><h2>Alert destinations</h2></div>
          <p className="muted" style={{ marginBottom: 12 }}>
            Push new alerts to Slack, a webhook, or email. For Slack, paste an{" "}
            <b>Incoming Webhook</b> URL; for a webhook, any HTTPS endpoint that accepts a JSON POST;
            for email, an address (email requires Resend to be configured on the server). Delivery
            runs on the monitoring tick.
          </p>

          {dests.length > 0 && (
            <table className="bl-table" style={{ marginBottom: 14 }}>
              <thead>
                <tr><th>Name</th><th>Type</th><th>Target</th><th>Frequency</th><th>Last</th><th></th></tr>
              </thead>
              <tbody>
                {dests.map((d) => (
                  <tr key={d.id} style={d.enabled ? undefined : { opacity: 0.5 }}>
                    <td>{d.name || <span className="muted">-</span>}</td>
                    <td>{d.kind}</td>
                    <td className="mono sm" style={{ maxWidth: 220, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={d.target}>{d.target}</td>
                    <td>
                      <select
                        value={d.digest}
                        title="Delivery frequency"
                        onChange={(e) => void changeDigest(d, e.target.value)}
                      >
                        <option value="immediate">Immediate</option>
                        <option value="hourly">Hourly digest</option>
                        <option value="daily">Daily digest</option>
                      </select>
                    </td>
                    <td className="sm">
                      {d.last_status === "ok" ? (
                        <span className="badge" style={{ background: "#dcfce7", color: "#166534" }}>ok</span>
                      ) : d.last_status === "error" ? (
                        <span className="badge" style={{ background: "#fee2e2", color: "#991b1b" }} title={d.last_error ?? ""}>error</span>
                      ) : (
                        <span className="muted">-</span>
                      )}
                    </td>
                    <td style={{ whiteSpace: "nowrap", textAlign: "right" }}>
                      <button className="btn sm" disabled={dbusy} onClick={() => void testDest(d)}>Test</button>{" "}
                      <button className="btn sm" onClick={() => void toggleDest(d)}>{d.enabled ? "Disable" : "Enable"}</button>{" "}
                      <button className="btn sm" style={{ color: "#b91c1c", borderColor: "#fecaca" }} onClick={() => void delDest(d)}>Delete</button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}

          <div style={{ display: "flex", gap: 10, flexWrap: "wrap", alignItems: "flex-end" }}>
            <label className="auth-field" style={{ margin: 0 }}>
              <span>Type</span>
              <select value={dkind} onChange={(e) => setDkind(e.target.value)}>
                <option value="slack">Slack</option>
                <option value="webhook">Webhook</option>
                <option value="email">Email</option>
              </select>
            </label>
            <label className="auth-field" style={{ margin: 0, flex: "1 1 320px" }}>
              <span>{dkind === "email" ? "Email address" : "Destination URL"}</span>
              <input
                value={dtarget}
                onChange={(e) => setDtarget(e.target.value)}
                placeholder={
                  dkind === "email"
                    ? "alerts@yourcompany.com"
                    : dkind === "slack"
                      ? "https://hooks.slack.com/services/…"
                      : "https://your-app.com/webhooks/backlio"
                }
              />
            </label>
            <label className="auth-field" style={{ margin: 0 }}>
              <span>Name (optional)</span>
              <input value={dname} onChange={(e) => setDname(e.target.value)} placeholder="Team Slack" />
            </label>
            <label className="auth-field" style={{ margin: 0 }}>
              <span>Frequency</span>
              <select value={ddigest} onChange={(e) => setDdigest(e.target.value)}>
                <option value="immediate">Immediate</option>
                <option value="hourly">Hourly digest</option>
                <option value="daily">Daily digest</option>
              </select>
            </label>
            <button className="btn primary" disabled={dbusy} onClick={() => void addDest()}>Add destination</button>
          </div>
          {dmsg && <p className="muted" style={{ marginTop: 10 }}>{dmsg}</p>}
        </section>
        </>
      )}
    </main>
  );
}
