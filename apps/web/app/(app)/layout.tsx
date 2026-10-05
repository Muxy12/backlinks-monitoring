"use client";

import { useEffect, useState } from "react";
import { usePathname, useRouter } from "next/navigation";
import { api, type MeResponse } from "@/lib/api";
import { selectedOrg, setSelectedOrg } from "@/lib/workspace";
import { ToastHost } from "./toast";
import { TrialBanner } from "./TrialBanner";
import { Icon } from "./icons";
import "./dashboard/dashboard.css";
import "./shell.css";

// Shared shell for every authenticated page (route group "(app)", so URLs are
// unchanged). It owns the cross-cutting navigation — a vertical sidebar on
// desktop, a slide-in drawer on mobile — so pages only render their own title
// and content, never the nav.
type Link = { href: string; label: string; icon: string; badge?: string };
const SECTIONS: { title: string; links: Link[] }[] = [
  {
    title: "Monitor",
    links: [
      { href: "/dashboard", label: "Dashboard", icon: "dashboard" },
      { href: "/projects", label: "Projects", icon: "projects" },
      { href: "/graph", label: "Graph", icon: "graph" },
    ],
  },
  {
    title: "Grow",
    links: [
      { href: "/competitors", label: "Competitors", icon: "competitors" },
      { href: "/operations", label: "Operations", icon: "operations" },
      { href: "/import", label: "Import", icon: "import" },
    ],
  },
  {
    title: "Workspace",
    links: [
      { href: "/clients", label: "Clients", icon: "clients" },
      { href: "/team", label: "Team", icon: "team" },
    ],
  },
];

export default function AppLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [me, setMe] = useState<MeResponse | null>(null);
  const isAdmin = !!me?.user.is_admin;
  const campaignsBeta = !!me?.features?.campaigns_beta;
  // Surface the link-building workflow under "Grow" only when the BETA is on.
  const sections = SECTIONS.map((s) =>
    s.title === "Grow" && campaignsBeta
      ? {
          ...s,
          links: [
            ...s.links,
            { href: "/campaigns", label: "Link Building", icon: "campaigns", badge: "BETA" },
          ],
        }
      : s,
  );
  const orgs = me?.organizations ?? [];
  const current = selectedOrg(orgs);
  const workspace = current?.name ?? null;

  function switchOrg(id: string) {
    setSelectedOrg(id);
    // A full reload is the simplest way to re-scope every page to the new org.
    window.location.href = "/dashboard";
  }

  useEffect(() => {
    let alive = true;
    api.me().then((res) => {
      if (alive && res.ok) setMe(res.data);
    });
    return () => {
      alive = false;
    };
  }, []);

  const isActive = (href: string) => pathname === href || pathname?.startsWith(href + "/");

  const brand = (
    <a className="side-brand" href="/dashboard" onClick={() => setOpen(false)}>
      <img
        className="side-logo"
        src="https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/logo-linkpulz.webp"
        alt=""
        width="26"
        height="26"
      />
      <span className="side-brand-text">
        LinkPulz
        {workspace && <span className="side-workspace">{workspace}</span>}
      </span>
    </a>
  );

  const navLink = (l: Link) => (
    <a
      key={l.href}
      href={l.href}
      className={`side-link${isActive(l.href) ? " active" : ""}`}
      aria-current={isActive(l.href) ? "page" : undefined}
      onClick={() => setOpen(false)}
    >
      <span className="side-ico"><Icon name={l.icon} /></span>
      {l.label}
      {l.badge && (
        <span
          style={{
            marginLeft: "auto",
            fontSize: "9px",
            fontWeight: 700,
            letterSpacing: "0.04em",
            padding: "1px 5px",
            borderRadius: "999px",
            background: "#2f6bff",
            color: "#fff",
          }}
        >
          {l.badge}
        </span>
      )}
    </a>
  );

  const initial = (me?.user.name || me?.user.email || "?").trim().charAt(0).toUpperCase();

  return (
    <div className="shell">
      <aside className={`sidebar${open ? " open" : ""}`}>
        {brand}
        {orgs.length > 1 && (
          <select
            className="side-switcher"
            value={current?.id ?? ""}
            onChange={(e) => switchOrg(e.target.value)}
            title="Switch workspace"
            aria-label="Switch workspace"
          >
            {orgs.map((o) => (
              <option key={o.id} value={o.id}>{o.name}</option>
            ))}
          </select>
        )}
        <nav className="side-nav" aria-label="Sections">
          {sections.map((s) => (
            <div className="side-group" key={s.title}>
              <div className="side-group-title">{s.title}</div>
              {s.links.map(navLink)}
            </div>
          ))}
          {isAdmin && (
            <div className="side-group">
              <div className="side-group-title">Admin</div>
              {navLink({ href: "/admin", label: "Admin", icon: "admin" })}
            </div>
          )}
        </nav>
        <div className="side-spacer" />
        <div className="side-group">
          {navLink({ href: "/feedback", label: "Feedback", icon: "feedback" })}
          {navLink({ href: "/settings/billing", label: "Billing", icon: "billing" })}
          {navLink({ href: "/settings/notifications", label: "Settings", icon: "settings" })}
        </div>

        {me && (
          <div className="side-user" title={me.user.email}>
            <span className="side-avatar">{initial}</span>
            <span className="side-user-meta">
              <span className="side-user-name">{me.user.name || me.user.email}</span>
              {me.user.name && <span className="side-user-email">{me.user.email}</span>}
            </span>
            <button
              className="side-user-logout"
              title="Log out"
              aria-label="Log out"
              onClick={async () => {
                await api.logout();
                router.push("/login");
              }}
            >
              <Icon name="logout" size={15} />
            </button>
          </div>
        )}
      </aside>

      {open && <div className="shell-scrim" onClick={() => setOpen(false)} />}

      <div className="shell-main">
        <div className="mobile-bar">
          <button className="hamburger" onClick={() => setOpen((v) => !v)} aria-label="Open menu">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden="true">
              <line x1="3" y1="6" x2="21" y2="6" /><line x1="3" y1="12" x2="21" y2="12" /><line x1="3" y1="18" x2="21" y2="18" />
            </svg>
          </button>
          {brand}
        </div>
        <TrialBanner orgId={current?.id ?? null} />
        {children}
      </div>
      <ToastHost />
    </div>
  );
}
