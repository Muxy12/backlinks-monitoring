"use client";

import { useEffect, useState } from "react";
import { usePathname, useRouter } from "next/navigation";
import { api, type MeResponse } from "@/lib/api";
import { ToastHost } from "./toast";
import "./dashboard/dashboard.css";
import "./shell.css";

// Shared shell for every authenticated page (route group "(app)", so URLs are
// unchanged). It owns the cross-cutting navigation — a vertical sidebar on
// desktop, a slide-in drawer on mobile — so pages only render their own title
// and content, never the nav.
type Link = { href: string; label: string; icon: string };
const SECTIONS: { title: string; links: Link[] }[] = [
  {
    title: "Monitor",
    links: [
      { href: "/dashboard", label: "Dashboard", icon: "📊" },
      { href: "/projects", label: "Projects", icon: "🗂️" },
      { href: "/graph", label: "Graph", icon: "🕸️" },
    ],
  },
  {
    title: "Grow",
    links: [
      { href: "/competitors", label: "Competitors", icon: "🥊" },
      { href: "/operations", label: "Operations", icon: "🎯" },
      { href: "/import", label: "Import", icon: "⬆️" },
    ],
  },
  {
    title: "Workspace",
    links: [
      { href: "/clients", label: "Clients", icon: "🏢" },
      { href: "/team", label: "Team", icon: "👥" },
    ],
  },
];

export default function AppLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [me, setMe] = useState<MeResponse | null>(null);
  const isAdmin = !!me?.user.is_admin;
  const workspace = me?.organizations[0]?.name ?? null;

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
      <span className="brand-mark" />
      <span className="side-brand-text">
        Backlio
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
      <span className="side-ico">{l.icon}</span>
      {l.label}
    </a>
  );

  const initial = (me?.user.name || me?.user.email || "?").trim().charAt(0).toUpperCase();

  return (
    <div className="shell">
      <aside className={`sidebar${open ? " open" : ""}`}>
        {brand}
        <nav className="side-nav" aria-label="Sections">
          {SECTIONS.map((s) => (
            <div className="side-group" key={s.title}>
              <div className="side-group-title">{s.title}</div>
              {s.links.map(navLink)}
            </div>
          ))}
          {isAdmin && (
            <div className="side-group">
              <div className="side-group-title">Admin</div>
              {navLink({ href: "/admin", label: "Admin", icon: "🛡️" })}
            </div>
          )}
        </nav>
        <div className="side-spacer" />
        <div className="side-group">
          {navLink({ href: "/settings/billing", label: "Billing", icon: "💳" })}
          {navLink({ href: "/settings/notifications", label: "Settings", icon: "⚙️" })}
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
              ↩
            </button>
          </div>
        )}
      </aside>

      {open && <div className="shell-scrim" onClick={() => setOpen(false)} />}

      <div className="shell-main">
        <div className="mobile-bar">
          <button className="hamburger" onClick={() => setOpen((v) => !v)} aria-label="Open menu">
            ☰
          </button>
          {brand}
        </div>
        {children}
      </div>
      <ToastHost />
    </div>
  );
}
