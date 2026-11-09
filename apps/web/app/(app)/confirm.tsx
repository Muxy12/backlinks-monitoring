"use client";

import { useEffect, useRef, useState } from "react";
import "./confirm.css";

// Imperative confirm/prompt dialog, mirroring the toast system (toast.tsx):
// `confirmDialog()` dispatches a CustomEvent that <ConfirmHost/> (mounted once
// in the app shell) renders, resolving a promise when the user acts. This
// replaces window.confirm()/window.prompt(), which THROW "not supported" in
// embedded/webview browsers (and are increasingly restricted in sandboxed
// contexts) — there, a native confirm aborts the whole action silently.
//
// Drop-in usage:
//   if (!(await confirmDialog({ message: "Delete X?", danger: true }))) return;
//   // type-to-confirm (replaces prompt):
//   const ok = await confirmDialog({ message: "…", requireText: slug });

export type ConfirmOptions = {
  title?: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
  /** When set, the user must type this exact value to enable the confirm button. */
  requireText?: string;
};

type Payload = ConfirmOptions & { resolve: (ok: boolean) => void };

export function confirmDialog(opts: ConfirmOptions): Promise<boolean> {
  if (typeof window === "undefined") return Promise.resolve(false);
  return new Promise<boolean>((resolve) => {
    window.dispatchEvent(new CustomEvent("backlio-confirm", { detail: { ...opts, resolve } }));
  });
}

export function ConfirmHost() {
  const [payload, setPayload] = useState<Payload | null>(null);
  const [typed, setTyped] = useState("");
  const inputRef = useRef<HTMLInputElement | null>(null);

  function close(ok: boolean) {
    setPayload((cur) => {
      cur?.resolve(ok);
      return null;
    });
    setTyped("");
  }

  useEffect(() => {
    const onConfirm = (e: Event) => {
      const detail = (e as CustomEvent).detail as Payload;
      setTyped("");
      setPayload((prev) => {
        // If one is somehow already open, resolve it false before replacing.
        prev?.resolve(false);
        return detail;
      });
    };
    window.addEventListener("backlio-confirm", onConfirm);
    return () => window.removeEventListener("backlio-confirm", onConfirm);
  }, []);

  useEffect(() => {
    if (!payload) return;
    inputRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [payload]);

  if (!payload) return null;

  const reqUnmet = !!payload.requireText && typed.trim() !== payload.requireText;

  return (
    <div className="confirm-backdrop" onClick={() => close(false)}>
      <div
        className="confirm-card"
        role="dialog"
        aria-modal="true"
        aria-label={payload.title ?? "Confirm"}
        onClick={(e) => e.stopPropagation()}
      >
        <h2>{payload.title ?? "Please confirm"}</h2>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            if (!reqUnmet) close(true);
          }}
        >
          <p className="confirm-msg">{payload.message}</p>
          {payload.requireText && (
            <label className="confirm-field">
              <span>
                Type <code>{payload.requireText}</code> to confirm
              </span>
              <input
                ref={inputRef}
                value={typed}
                onChange={(e) => setTyped(e.target.value)}
                autoComplete="off"
                spellCheck={false}
              />
            </label>
          )}
          <div className="confirm-actions">
            <button type="button" className="btn sm" onClick={() => close(false)}>
              {payload.cancelLabel ?? "Cancel"}
            </button>
            <button
              type="submit"
              className={`btn sm ${payload.danger ? "danger" : "primary"}`}
              disabled={reqUnmet}
            >
              {payload.confirmLabel ?? "Confirm"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
