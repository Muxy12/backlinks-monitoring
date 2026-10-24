import { useEffect } from "react";

// Accessible drawer focus: when a drawer opens, move focus into it; when it
// closes, restore focus to whatever opened it. Pairs with an Escape handler +
// role="dialog"/aria-modal so the drawers are real modal dialogs.
export function useDrawerFocus(open: boolean, ref: { current: HTMLElement | null }) {
  useEffect(() => {
    if (!open) return;
    const opener = document.activeElement as HTMLElement | null;
    ref.current?.focus();
    return () => opener?.focus?.();
  }, [open, ref]);
}
