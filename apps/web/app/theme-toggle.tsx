"use client";

import { useEffect, useState } from "react";
import "./theme-toggle.css";

const KEY = "backlio_theme";
type Theme = "light" | "dark";

function current(): Theme {
  if (typeof document === "undefined") return "dark";
  return document.documentElement.getAttribute("data-theme") === "light" ? "light" : "dark";
}

// Floating light/dark switch. The actual theme is applied before paint by the
// inline script in the root layout (reads localStorage, else the OS preference),
// so this button only reflects and flips the current value. Rendered site-wide
// (incl. the static marketing homepage) from the root layout.
export function ThemeToggle() {
  const [theme, setTheme] = useState<Theme>("dark");
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setTheme(current());
    setMounted(true);
  }, []);

  const toggle = () => {
    const next: Theme = theme === "light" ? "dark" : "light";
    document.documentElement.setAttribute("data-theme", next);
    try {
      localStorage.setItem(KEY, next);
    } catch {
      /* private mode, theme still applies for this session */
    }
    setTheme(next);
  };

  // Avoid a hydration mismatch: render nothing until we've read the real theme.
  if (!mounted) return null;

  const goingTo = theme === "light" ? "dark" : "light";
  return (
    <button
      type="button"
      className="theme-toggle"
      onClick={toggle}
      aria-label={`Switch to ${goingTo} mode`}
      title={`Switch to ${goingTo} mode`}
    >
      {theme === "light" ? <MoonIcon /> : <SunIcon />}
    </button>
  );
}

// Inline SVG (Lucide) so this root-level toggle stays self-contained on the
// static marketing pages too, no import from the (app) route group.
function svgProps(size: number) {
  return {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 2,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    "aria-hidden": true,
    focusable: false as const,
  };
}
function SunIcon() {
  return (
    <svg {...svgProps(18)}>
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" />
    </svg>
  );
}
function MoonIcon() {
  return (
    <svg {...svgProps(18)}>
      <path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z" />
    </svg>
  );
}
