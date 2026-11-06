"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api, orgApi, type ApiKeyView, type CreatedKey, type MeResponse } from "@/lib/api";
import { selectedOrgId } from "@/lib/workspace";
import "@/app/auth.css";
import "../../dashboard/dashboard.css";
import "../../operations/operations.css";

// All selectable scopes (mirrors the API's VALID_SCOPES). Read scopes are on by
// default; write:discovery is opt-in.
const SCOPES: { id: string; label: string }[] = [
  { id: "read:portfolio", label: "Read portfolio (clients, projects, links, alerts)" },
  { id: "read:discovery", label: "Read discovery (spots, scores)" },
  { id: "read:geo", label: "Read GEO (AI citations)" },
  { id: "read:marketplace", label: "Read marketplace (offers)" },
  { id: "write:discovery", label: "Trigger discovery / GEO (write)" },
];
const DEFAULT_SCOPES = ["read:portfolio", "read:discovery", "read:geo", "read:marketplace"];

// Settings → API keys. Create, list and revoke the lp_live_ Bearer keys used by
// the MCP server and the keyed REST API. The secret is shown exactly once.
export default function ApiKeysPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [keys, setKeys] = useState<ApiKeyView[]>([]);
  const [name, setName] = useState("");
  const [scopes, setScopes] = useState<string[]>(DEFAULT_SCOPES);
  const [created, setCreated] = useState<CreatedKey | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    api.me().then((res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      setMe(res.data);
      setOrgId(selectedOrgId(res.data.organizations));
    });
  }, [router]);

  const load = useCallback(async (org: string) => {
    const res = await orgApi.apiKeys(org);
    if (res.ok) setKeys(res.data.keys);
  }, []);

  useEffect(() => {
    if (orgId) void load(orgId);
  }, [orgId, load]);

  function toggleScope(id: string) {
    setScopes((cur) => (cur.includes(id) ? cur.filter((s) => s !== id) : [...cur, id]));
  }

  async function create() {
    if (!orgId) return;
    const trimmed = name.trim();
    if (!trimmed) return;
    setErr(null);
    setBusy(true);
    const res = await orgApi.createApiKey(orgId, trimmed, scopes);
    setBusy(false);
    if (!res.ok) {
      setErr(res.detail || "Could not create the key.");
      return;
    }
    setCreated(res.data);
    setName("");
    await load(orgId);
  }

  async function revoke(id: string) {
    if (!orgId) return;
    const res = await orgApi.revokeApiKey(orgId, id);
    if (res.ok) await load(orgId);
  }

  if (!me) return null;

  const active = keys.filter((k) => !k.revoked_at);
  const revoked = keys.filter((k) => k.revoked_at);

  return (
    <>
      <section className="panel">
        <h2>API keys</h2>
        <p className="muted" style={{ marginBottom: 12 }}>
          Keys authenticate the MCP server and the keyed REST API as this workspace. A key can only
          read what its scopes allow; the secret is shown once, so copy it now. Keep keys private —
          anyone with a key can act for this workspace within its scopes.
        </p>

        {created && (
          <div className="panel" style={{ borderColor: "var(--live)", marginBottom: 12 }}>
            <p style={{ marginTop: 0, fontWeight: 650 }}>
              Key <span className="mono">{created.name}</span> created — copy it now, it won&rsquo;t
              be shown again:
            </p>
            <pre className="mono" style={{ userSelect: "all", whiteSpace: "pre-wrap", wordBreak: "break-all" }}>
              {created.secret}
            </pre>
            <button className="btn sm" onClick={() => setCreated(null)}>
              Done
            </button>
          </div>
        )}

        {err && (
          <div className="auth-error" role="alert" style={{ marginBottom: 10 }}>
            {err}
          </div>
        )}

        <form
          onSubmit={(e) => {
            e.preventDefault();
            void create();
          }}
        >
          <label className="muted" htmlFor="key-name">
            Name
          </label>
          <input
            id="key-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. Claude MCP, CI reporting…"
            style={{ display: "block", marginBottom: 10 }}
          />
          <fieldset style={{ border: "none", padding: 0, margin: "0 0 10px" }}>
            <legend className="muted" style={{ padding: 0 }}>
              Scopes
            </legend>
            {SCOPES.map((s) => (
              <label key={s.id} style={{ display: "block", fontSize: 14 }}>
                <input
                  type="checkbox"
                  checked={scopes.includes(s.id)}
                  onChange={() => toggleScope(s.id)}
                />{" "}
                <span className="mono">{s.id}</span> — {s.label}
              </label>
            ))}
          </fieldset>
          <button className="btn primary" type="submit" disabled={busy || !name.trim()}>
            {busy ? "Creating…" : "Create key"}
          </button>
        </form>
      </section>

      <section className="panel">
        <h2>Your keys</h2>
        {active.length === 0 ? (
          <p className="muted">No active keys yet.</p>
        ) : (
          <div className="bl-scroll">
            <table className="bl-table">
              <thead>
                <tr>
                  <th>Name</th>
                  <th>Prefix</th>
                  <th>Scopes</th>
                  <th>Last used</th>
                  <th>Created</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {active.map((k) => (
                  <tr key={k.id}>
                    <td>{k.name}</td>
                    <td className="mono">{k.prefix}…</td>
                    <td className="muted" style={{ fontSize: 13 }}>
                      {k.scopes.join(", ") || "—"}
                    </td>
                    <td>{k.last_used_at ? new Date(k.last_used_at).toLocaleDateString() : "never"}</td>
                    <td>{new Date(k.created_at).toLocaleDateString()}</td>
                    <td>
                      <button className="btn sm ghost" onClick={() => void revoke(k.id)}>
                        Revoke
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        {revoked.length > 0 && (
          <p className="muted" style={{ marginTop: 10, fontSize: 13 }}>
            {revoked.length} revoked key{revoked.length === 1 ? "" : "s"} hidden.
          </p>
        )}
      </section>
    </>
  );
}
