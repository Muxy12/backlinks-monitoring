"use client";

import { useEffect, useState } from "react";
import { usePathname, useRouter } from "next/navigation";
import { api } from "@/lib/api";
import "./dashboard/dashboard.css";
import "./shell.css";

// Shared shell for every authenticated page (route group "(app)", so URLs are
// unchanged). It owns the cross-cutting navigation — a vertical sidebar on
// desktop, a slide-in drawer on mobile — so pages only render their own title
// and content, never the nav.
const LINKS: { href: string; label: string; icon: string }[] = [
  { href: "/dashboard", label: "Dashboard", icon: "📊" },
  { href: "/projects", label: "Projects", icon: "🗂️" },
  { href: "/clients", label: "Clients", icon: "🏢" },
  { href: "/competitors", label: "Competitors", icon: "🥊" },
  { href: "/graph", label: "Graph", icon: "🕸️" },
  { href: "/operations", label: "Operations", icon: "🎯" },
  { href: "/import", label: "Import", icon: "⬆️" },
  { href: "/team", label: "Team", icon: "👥" },
];

export default function AppLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [isAdmin, setIsAdmin] = useState(false);

  // Reveal the Admin link only for platform admins (best-effort; the /admin page
  // and API enforce access regardless of what the nav shows).
  useEffect(() => {
    let alive = true;
    api.me().then((res) => {
      if (alive && res.ok) setIsAdmin(!!res.data.user.is_admin);
    });
    return () => {
      alive = false;
    };
  }, []);

  const isActive = (href: string) => pathname === href || pathname?.startsWith(href + "/");

  const brand = (
    <a className="side-brand" href="/dashboard" onClick={() => setOpen(false)}>
      <span className="brand-mark" />
      Backlio
    </a>
  );

  return (
    <div className="shell">
      <aside className={`sidebar${open ? " open" : ""}`}>
        {brand}
        <nav className="side-nav" aria-label="Sections">
          {LINKS.map((l) => (
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
          ))}
          {isAdmin && (
            <a
              href="/admin"
              className={`side-link${isActive("/admin") ? " active" : ""}`}
              aria-current={isActive("/admin") ? "page" : undefined}
              onClick={() => setOpen(false)}
            >
              <span className="side-ico">🛡️</span>
              Admin
            </a>
          )}
        </nav>
        <div className="side-spacer" />
        <a
          href="/settings/notifications"
          className={`side-link${isActive("/settings") ? " active" : ""}`}
          onClick={() => setOpen(false)}
        >
          <span className="side-ico">⚙️</span>
          Settings
        </a>
        <button
          className="side-link side-logout"
          onClick={async () => {
            await api.logout();
            router.push("/login");
          }}
        >
          <span className="side-ico">↩</span>
          Log out
        </button>
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
    </div>
  );
}
