"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { orgApi, type NotificationsResponse } from "@/lib/api";
import { useDrawerFocus } from "@/lib/useDrawerFocus";
import { Icon } from "@/app/(app)/icons";

// Workspace notifications, as a global shell control. Previously this bell and
// its drawer lived only on the backlinks page (the legacy all-in-one dashboard);
// notifications are cross-cutting, so the bell belongs in the app shell and is
// reachable from every page.
export function NotificationsBell({ orgId }: { orgId: string | null }) {
  const [notifs, setNotifs] = useState<NotificationsResponse | null>(null);
  const [open, setOpen] = useState(false);
  const drawerRef = useRef<HTMLElement | null>(null);
  useDrawerFocus(open, drawerRef);

  const load = useCallback(async (org: string) => {
    const res = await orgApi.notifications(org);
    if (res.ok) setNotifs(res.data);
  }, []);

  useEffect(() => {
    if (orgId) void load(orgId);
  }, [orgId, load]);

  // Close the drawer on Escape.
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open]);

  async function markRead(id: string) {
    if (!orgId) return;
    const res = await orgApi.markNotificationRead(orgId, id);
    if (res.ok) await load(orgId);
  }

  return (
    <>
      <button
        className="btn bell"
        onClick={() => setOpen((v) => !v)}
        aria-label="Notifications"
        aria-expanded={open}
        aria-haspopup="dialog"
      >
        <Icon name="bell" size={17} />
        {notifs && notifs.unread > 0 && <span className="bell-badge">{notifs.unread}</span>}
      </button>
      {open && (
        <div className="drawer-backdrop" onClick={() => setOpen(false)}>
          <aside
            className="drawer"
            role="dialog"
            aria-modal="true"
            aria-label="Notifications"
            tabIndex={-1}
            ref={drawerRef}
            onClick={(e) => e.stopPropagation()}
          >
            <div className="panel-head">
              <h2>Notifications</h2>
              <div className="panel-actions">
                <a className="btn sm" href="/settings/notifications">Settings</a>
                <button className="btn" onClick={() => setOpen(false)}>Close</button>
              </div>
            </div>
            {!notifs || notifs.items.length === 0 ? (
              <p className="muted">No notifications yet.</p>
            ) : (
              <ul className="notif-list">
                {notifs.items.map((n) => (
                  <li key={n.id} className={n.read ? "notif read" : "notif"}>
                    <div>
                      <div className="notif-title">{n.title}</div>
                      {n.body && <div className="muted">{n.body}</div>}
                      <div className="muted mono">{new Date(n.created_at).toLocaleString()}</div>
                    </div>
                    {!n.read && (
                      <button className="btn sm" onClick={() => void markRead(n.id)}>Mark read</button>
                    )}
                  </li>
                ))}
              </ul>
            )}
          </aside>
        </div>
      )}
    </>
  );
}
