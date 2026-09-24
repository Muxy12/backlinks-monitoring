"use client";

import { type ReactNode } from "react";
import { useRouter } from "next/navigation";
import { api } from "../lib/api";

// Shared back-office navigation. Every authenticated page renders the same set
// of section links with the current one highlighted, plus Log out. Pages can
// inject extra controls (e.g. the dashboard's notifications bell) via children,
// which render just before Log out.
const LINKS: { key: string; label: string; href: string }[] = [
  { key: "dashboard", label: "Dashboard", href: "/dashboard" },
  { key: "projects", label: "Projects", href: "/projects" },
  { key: "clients", label: "Clients", href: "/clients" },
  { key: "operations", label: "Operations", href: "/operations" },
  { key: "import", label: "Import", href: "/import" },
  { key: "team", label: "Team", href: "/team" },
];

export function AppNav({ active, children }: { active?: string; children?: ReactNode }) {
  const router = useRouter();
  return (
    <nav className="head-actions" aria-label="Sections">
      {LINKS.map((l) => (
        <a
          key={l.key}
          className={`btn${active === l.key ? " active" : ""}`}
          href={l.href}
          aria-current={active === l.key ? "page" : undefined}
        >
          {l.label}
        </a>
      ))}
      {children}
      <button
        className="btn"
        onClick={async () => {
          await api.logout();
          router.push("/login");
        }}
      >
        Log out
      </button>
    </nav>
  );
}
