"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api, orgApi, type ClientView, type MeResponse } from "@/lib/api";
import "@/app/auth.css";
import "../dashboard/dashboard.css";
import "../operations/operations.css";

export default function ClientsPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [clients, setClients] = useState<ClientView[]>([]);
  const [name, setName] = useState("");
  const [company, setCompany] = useState("");
  const [website, setWebsite] = useState("");
  const [contactName, setContactName] = useState("");
  const [contactEmail, setContactEmail] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [editId, setEditId] = useState<string | null>(null);

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
    const res = await orgApi.clients(org);
    if (res.ok) setClients(res.data);
  }, []);

  useEffect(() => {
    if (orgId) void load(orgId);
  }, [orgId, load]);

  function resetForm() {
    setEditId(null);
    setName("");
    setCompany("");
    setWebsite("");
    setContactName("");
    setContactEmail("");
    setError(null);
  }

  function startEdit(c: ClientView) {
    setEditId(c.id);
    setName(c.name);
    setCompany(c.company_name ?? "");
    setWebsite(c.website ?? "");
    setContactName(c.contact_name ?? "");
    setContactEmail(c.contact_email ?? "");
    setError(null);
  }

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    if (!orgId) return;
    const body = {
      name: name.trim(),
      company_name: company.trim() || undefined,
      website: website.trim() || undefined,
      contact_name: contactName.trim() || undefined,
      contact_email: contactEmail.trim() || undefined,
    };
    setBusy(true);
    const res = editId
      ? await orgApi.updateClient(orgId, editId, body)
      : await orgApi.createClient(orgId, body);
    setBusy(false);
    if (!res.ok) {
      setError(res.detail);
      return;
    }
    resetForm();
    void load(orgId);
  }

  async function remove(c: ClientView) {
    if (!orgId) return;
    if (!confirm(`Delete client "${c.name}"? Projects linked to it will be unassigned.`)) return;
    const res = await orgApi.deleteClient(orgId, c.id);
    if (!res.ok) {
      setError(res.detail);
      return;
    }
    if (editId === c.id) resetForm();
    void load(orgId);
  }

  if (loading) return <div className="auth-wrap"><p style={{ color: "var(--muted)" }}>Loading…</p></div>;
  if (!me) return null;

  return (
    <main className="dash">
      <header className="page-head">
        <h1>Clients</h1>
        <p className="muted">The companies you build links for.</p>
      </header>

      {!orgId ? (
        <p className="muted">Create a workspace first.</p>
      ) : (
        <div className="ops-grid">
          <section className="panel">
            <div className="panel-head"><h2>Clients ({clients.length})</h2></div>
            {clients.length === 0 ? (
              <p className="muted">No clients yet.</p>
            ) : (
              <table className="bl-table">
                <thead>
                  <tr><th>Name</th><th>Company</th><th>Contact</th><th></th></tr>
                </thead>
                <tbody>
                  {clients.map((c) => (
                    <tr key={c.id} style={editId === c.id ? { background: "color-mix(in srgb, var(--accent) 10%, transparent)" } : undefined}>
                      <td>{c.name}</td>
                      <td>{c.company_name ?? "—"}</td>
                      <td className="mono">{c.contact_email ?? c.contact_name ?? "—"}</td>
                      <td style={{ textAlign: "right", whiteSpace: "nowrap" }}>
                        <button className="btn sm" onClick={() => startEdit(c)}>Edit</button>{" "}
                        <button
                          className="btn sm"
                          style={{ color: "#b91c1c", borderColor: "#fecaca" }}
                          onClick={() => void remove(c)}
                        >
                          Delete
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </section>

          <section className="panel">
            <div className="panel-head"><h2>{editId ? "Edit client" : "New client"}</h2></div>
            <form onSubmit={submit} className="ops-form">
              {error && <div className="auth-error">{error}</div>}
              <label className="auth-field">
                <span>Name</span>
                <input value={name} onChange={(e) => setName(e.target.value)} required />
              </label>
              <label className="auth-field">
                <span>Company (optional)</span>
                <input value={company} onChange={(e) => setCompany(e.target.value)} />
              </label>
              <label className="auth-field">
                <span>Website (optional)</span>
                <input value={website} onChange={(e) => setWebsite(e.target.value)} placeholder="https://…" />
              </label>
              <label className="auth-field">
                <span>Contact name (optional)</span>
                <input value={contactName} onChange={(e) => setContactName(e.target.value)} />
              </label>
              <label className="auth-field">
                <span>Contact email (optional)</span>
                <input type="email" value={contactEmail} onChange={(e) => setContactEmail(e.target.value)} />
              </label>
              <button className="auth-btn" type="submit" disabled={busy}>
                {busy ? "Saving…" : editId ? "Save changes" : "Create client"}
              </button>
              {editId && (
                <button type="button" className="btn" style={{ marginTop: 8 }} onClick={resetForm}>
                  Cancel
                </button>
              )}
            </form>
          </section>
        </div>
      )}
    </main>
  );
}
