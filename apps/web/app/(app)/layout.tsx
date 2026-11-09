"use client";

import { useEffect, useState } from "react";
import { usePathname, useRouter } from "next/navigation";
import { api, type MeResponse } from "@/lib/api";
import { selectedOrg, setSelectedOrg } from "@/lib/workspace";
import { ToastHost } from "./toast";
import { ConfirmHost } from "./confirm";
import { TrialBanner } from "./TrialBanner";
import { Icon } from "./icons";
import { NotificationsBell } from "@/components/NotificationsBell";
import "./dashboard/dashboard.css";
import "./shell.css";

// Shared shell for every authenticated page (route group "(app)", so URLs are
// unchanged). It owns the cross-cutting navigation, a vertical sidebar on
// desktop, a slide-in drawer on mobile, so pages only render their own title
// and content, never the nav.
type Link = { href: string; label: string; icon: string; badge?: string };
// Sidebar IA for the AI link-building platform (refonte): lead with the
// acquisition funnel (my sites → find spots → campaigns), then monitoring, then
// workspace. Only real pages are linked; flag-gated surfaces are spliced below.
const SECTIONS: { title: string; links: Link[] }[] = [
  {
    title: "Acquire",
    links: [
      { href: "/projects", label: "My sites", icon: "projects" },
    ],
  },
  {
    title: "Monitor",
    links: [
      { href: "/dashboard", label: "Dashboard", icon: "dashboard" },
      { href: "/backlinks", label: "Backlinks", icon: "link" },
      { href: "/graph", label: "Graph", icon: "graph" },
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
  const spotFinder = !!me?.features?.spot_finder;
  const campaignsBeta = !!me?.features?.campaigns_beta;
  // "Acquire" = My sites → Spot Finder → Campaigns (the flag-gated funnel steps
  // splice in after My sites, in funnel order).
  const sections = SECTIONS.map((s) => {
    if (s.title !== "Acquire") return s;
    const links = [...s.links];
    if (spotFinder) links.push({ href: "/spots", label: "Spot Finder", icon: "signal" });
    if (campaignsBeta)
      links.push({ href: "/campaigns", label: "Campaigns", icon: "campaigns", badge: "BETA" });
    return { ...s, links };
  });
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
        src="https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/Logo_LinkPulz_en_n%C3%A9on_d%C3%A9grad%C3%A9.png"
        alt="LinkPulz"
        width={866}
        height={288}
      />
      {workspace && (
        <span className="side-brand-text">
          <span className="side-workspace">{workspace}</span>
        </span>
      )}
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
      {l.badge && <span className="side-badge">{l.badge}</span>}
    </a>
  );

  const initial = (me?.user.name || me?.user.email || "?").trim().charAt(0).toUpperCase();

  return (
    <div className="shell">
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      <aside id="app-sidebar" className={`sidebar${open ? " open" : ""}`}>
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
          {navLink({ href: "/settings", label: "Settings", icon: "settings" })}
        </div>

        {me && (
          <div className="side-user" title={me.user.email}>
            <span className="side-avatar">{initial}</span>
            <span className="side-user-meta">
              <span className="side-user-name">{me.user.name || me.user.email}</span>
              {me.user.name && <span className="side-user-email">{me.user.email}</span>}
            </span>
            <NotificationsBell orgId={current?.id ?? null} />
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

      <div className="shell-main" id="main" tabIndex={-1}>
        <div className="mobile-bar">
          <button
            className="hamburger"
            onClick={() => setOpen((v) => !v)}
            aria-label={open ? "Close menu" : "Open menu"}
            aria-expanded={open}
            aria-controls="app-sidebar"
          >
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
      <ConfirmHost />
    </div>
  );
}
