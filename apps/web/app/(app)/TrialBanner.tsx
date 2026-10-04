"use client";

import { useEffect, useState } from "react";
import { usePathname } from "next/navigation";
import { orgApi } from "@/lib/api";
import "./trial-banner.css";

// A subtle, global trial indicator shown in the app shell (docs/20 — P6). It
// reads the org's billing status and shows:
//   • "N days left in your free trial" while trialing (urgent styling ≤ 3 days),
//   • "Your free trial has ended — choose a plan" once access is inactive.
// It stays quiet for active subscriptions and on the billing page itself. No
// countdown timers or dark patterns — just a clear, honest nudge.
type Banner = {
  status: string;
  daysLeft?: number;
  accessActive: boolean;
};

export function TrialBanner({ orgId }: { orgId: string | null }) {
  const pathname = usePathname();
  const [b, setB] = useState<Banner | null>(null);

  useEffect(() => {
    if (!orgId) return;
    let alive = true;
    orgApi.billingStatus(orgId).then((r) => {
      if (alive && r.ok) {
        setB({
          status: r.data.status,
          daysLeft: r.data.trial_days_left,
          accessActive: r.data.access_active,
        });
      }
    });
    return () => {
      alive = false;
    };
  }, [orgId]);

  // Don't echo the banner on the page where you'd act on it.
  if (!b || pathname?.startsWith("/settings/billing")) return null;
  // Paid and active → nothing to nudge.
  if (b.status === "active") return null;

  // Trial expired or subscription inactive → firm but reassuring prompt. A
  // workspace with no subscription at all ("none") never had a trial, so we
  // don't claim one "ended".
  if (!b.accessActive) {
    const neverStarted = b.status === "none";
    return (
      <div className="trial-banner ended" role="status">
        <span className="tb-dot" />
        <span className="tb-text">
          {neverStarted ? (
            <><b>Choose a plan to start monitoring.</b> Your workspace is ready.</>
          ) : (
            <><b>Your free trial has ended.</b> Choose a plan to resume monitoring, your backlinks and history are safe.</>
          )}
        </span>
        <a className="tb-cta" href="/settings/billing">Choose a plan</a>
      </div>
    );
  }

  // Active trial → quiet countdown, firmer in the last three days.
  if (b.status === "trialing" && typeof b.daysLeft === "number") {
    const d = b.daysLeft;
    const urgent = d <= 3;
    return (
      <div className={`trial-banner${urgent ? " urgent" : ""}`} role="status">
        <span className="tb-dot" />
        <span className="tb-text">
          {d <= 1 ? "Last day of your free trial" : `${d} days left in your free trial`}
          <span className="tb-sub"> · no credit card charged</span>
        </span>
        <a className="tb-cta" href="/settings/billing">{urgent ? "Choose a plan" : "View plans"}</a>
      </div>
    );
  }
  return null;
}
