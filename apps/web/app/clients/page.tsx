"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api, orgApi, type ClientView, type MeResponse } from "../../lib/api";
import { AppNav } from "../../components/AppNav";
import "../auth.css";
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

  async function create(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    if (!orgId) return;
    setBusy(true);
    const res = await orgApi.createClient(orgId, {
      name: name.trim(),
      company_name: company.trim() || undefined,
      website: website.trim() || undefined,
      contact_name: contactName.trim() || undefined,
      contact_email: contactEmail.trim() || undefined,
    });
    setBusy(false);
    if (!res.ok) {
      setError(res.detail);
      return;
    }
    setName("");
    setCompany("");
    setWebsite("");
    setContactName("");
    setContactEmail("");
    void load(orgId);
  }

  async function remove(id: string) {
    if (!orgId) return;
    const res = await orgApi.deleteClient(orgId, id);
    if (res.ok) void load(orgId);
  }

  if (loading) return <div className="auth-wrap"><p style={{ color: "var(--muted)" }}>Loading…</p></div>;
  if (!me) return null;

  return (
    <main className="dash">
      <header className="dash-head">
        <div className="dash-id">
          <div className="brand"><span className="brand-mark" />Backlio</div>
          <h1>Clients</h1>
          <p className="muted">The companies you build links for.</p>
        </div>
        <AppNav active="clients" />
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
                    <tr key={c.id}>
                      <td>{c.name}</td>
                      <td>{c.company_name ?? "—"}</td>
                      <td className="mono">{c.contact_email ?? c.contact_name ?? "—"}</td>
                      <td style={{ textAlign: "right" }}>
                        <button className="btn sm" onClick={() => void remove(c.id)}>Delete</button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </section>

          <section className="panel">
            <div className="panel-head"><h2>New client</h2></div>
            <form onSubmit={create} className="ops-form">
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
              <button className="auth-btn" type="submit" disabled={busy}>{busy ? "Creating…" : "Create client"}</button>
            </form>
          </section>
        </div>
      )}
    </main>
  );
}
