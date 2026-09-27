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
      /* private mode — theme still applies for this session */
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
      {theme === "light" ? "🌙" : "☀️"}
    </button>
  );
}
