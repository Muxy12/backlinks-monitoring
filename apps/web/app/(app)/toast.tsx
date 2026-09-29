"use client";

import { useEffect, useState } from "react";

// Lightweight toast system with no context/provider: `toast()` dispatches a
// CustomEvent that <ToastHost/> (mounted once in the app shell) renders. Any
// client component can `import { toast }` and call it.
export type ToastKind = "success" | "error" | "info";

export function toast(message: string, kind: ToastKind = "info") {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent("backlio-toast", { detail: { message, kind } }));
}

type Item = { id: number; message: string; kind: ToastKind };

export function ToastHost() {
  const [items, setItems] = useState<Item[]>([]);

  useEffect(() => {
    let seq = 0;
    const onToast = (e: Event) => {
      const { message, kind } = (e as CustomEvent).detail as { message: string; kind: ToastKind };
      const id = ++seq;
      setItems((cur) => [...cur, { id, message, kind }]);
      // Errors linger a little longer than successes.
      const ttl = kind === "error" ? 6000 : 3500;
      window.setTimeout(() => setItems((cur) => cur.filter((t) => t.id !== id)), ttl);
    };
    window.addEventListener("backlio-toast", onToast);
    return () => window.removeEventListener("backlio-toast", onToast);
  }, []);

  if (items.length === 0) return null;

  return (
    <div className="toast-host" role="status" aria-live="polite">
      {items.map((t) => (
        <div key={t.id} className={`toast toast-${t.kind}`} onClick={() => setItems((c) => c.filter((x) => x.id !== t.id))}>
          <span className="toast-ico">{t.kind === "success" ? "✓" : t.kind === "error" ? "!" : "•"}</span>
          <span>{t.message}</span>
        </div>
      ))}
    </div>
  );
}
