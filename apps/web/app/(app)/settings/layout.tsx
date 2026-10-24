"use client";

import { usePathname } from "next/navigation";
import "../dashboard/dashboard.css";
import "./settings.css";

// Settings hub: one "Settings" page head + a tab bar that unifies the three
// settings pages (previously three disconnected routes scattered across the
// sidebar). Each child page renders only its own panels; the shell lives here.
const TABS = [
  { href: "/settings/notifications", label: "Notifications" },
  { href: "/settings/integrations", label: "Integrations" },
  { href: "/settings/billing", label: "Billing" },
];

export default function SettingsLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const isActive = (href: string) => pathname === href || pathname?.startsWith(href + "/");

  return (
    <main className="dash">
      <header className="page-head">
        <h1>Settings</h1>
      </header>
      <nav className="settings-tabs" aria-label="Settings sections">
        {TABS.map((t) => (
          <a
            key={t.href}
            href={t.href}
            className={`settings-tab${isActive(t.href) ? " active" : ""}`}
            aria-current={isActive(t.href) ? "page" : undefined}
          >
            {t.label}
          </a>
        ))}
      </nav>
      {children}
    </main>
  );
}
