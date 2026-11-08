"use client";

import { useEffect, useState } from "react";
import { api, orgApi } from "@/lib/api";

// First-run "Get started" checklist, marketplace-first: add a domain → find
// link spots across marketplaces → monitor what you land. Steps auto-check from
// real workspace state (projects + backlinks counts via billing/usage, alert
// destinations), so it is always accurate and never needs its own stored
// progress. The discovery step only appears when a marketplace source is
// configured; otherwise it falls back to the classic "add your backlinks"
// on-ramp. Dismissal is a per-browser convenience in localStorage; it hides
// itself once every step is done. Pairs with the coachmark Tour.

const DISMISS_KEY = "backlio_getting_started_dismissed";

type Step = {
  key: string;
  title: string;
  body: string;
  cta: string;
  href: string;
  done: boolean;
};

export function GettingStarted({ org, onStartTour }: { org: string; onStartTour: () => void }) {
  const [steps, setSteps] = useState<Step[] | null>(null);
  const [dismissed, setDismissed] = useState(true);

  useEffect(() => {
    try {
      setDismissed(localStorage.getItem(DISMISS_KEY) === "1");
    } catch {
      setDismissed(false);
    }
  }, []);

  useEffect(() => {
    let live = true;
    void (async () => {
      const [usage, dests, me] = await Promise.all([
        orgApi.billingUsage(org),
        orgApi.alertDestinations(org),
        api.me(),
      ]);
      if (!live) return;
      const projects = usage.ok ? usage.data.projects.used : 0;
      const backlinks = usage.ok ? usage.data.backlinks.used : 0;
      const hasAlerts = dests.ok ? dests.data.length > 0 : false;
      // Lead the second step with discovery when a marketplace source is live;
      // otherwise keep the direct "add your backlinks" on-ramp.
      const canDiscover = me.ok && (!!me.data.features?.marketplace || !!me.data.features?.spot_finder);
      const acquireStep: Step = canDiscover
        ? {
            key: "discover",
            title: "Find link spots to acquire",
            body: "Discover buyable placements across your marketplaces, relevant to your topics — then monitor the ones you land.",
            cta: "Find spots",
            href: "/spots",
            done: backlinks > 0,
          }
        : {
            key: "backlinks",
            title: "Add your backlinks",
            body: "Paste them or upload a CSV, LinkPulz re-checks each one on a schedule.",
            cta: "Add backlinks",
            href: "/backlinks",
            done: backlinks > 0,
          };
      setSteps([
        {
          key: "project",
          title: "Add your domain",
          body: "Add the website whose link profile you want to grow and monitor.",
          cta: "Add a domain",
          href: "/projects",
          done: projects > 0,
        },
        acquireStep,
        {
          key: "alerts",
          title: "Set up alerts",
          body: "Get an email, Slack or webhook the moment a link is lost or changes.",
          cta: "Set up alerts",
          href: "/settings/notifications",
          done: hasAlerts,
        },
      ]);
    })();
    return () => {
      live = false;
    };
  }, [org]);

  if (dismissed || !steps) return null;
  const doneCount = steps.filter((s) => s.done).length;
  if (doneCount === steps.length) return null; // all set → nothing to nag about

  const dismiss = () => {
    try {
      localStorage.setItem(DISMISS_KEY, "1");
    } catch {
      /* private mode */
    }
    setDismissed(true);
  };

  return (
    <section className="gs-card" data-tour="checklist" aria-label="Getting started">
      <div className="gs-head">
        <div>
          <h2>
            Get started <span className="gs-count">{doneCount}/{steps.length}</span>
          </h2>
          <p className="muted gs-sub">Three quick steps to find links and start monitoring them.</p>
        </div>
        <div className="gs-head-actions">
          <button className="btn sm ghost" onClick={onStartTour}>
            Take the tour
          </button>
          <button className="gs-dismiss" aria-label="Dismiss checklist" onClick={dismiss}>
            ×
          </button>
        </div>
      </div>
      <ol className="gs-steps">
        {steps.map((s, i) => (
          <li key={s.key} className={s.done ? "gs-step done" : "gs-step"}>
            <span className="gs-mark" aria-hidden="true">
              {s.done ? "✓" : i + 1}
            </span>
            <div className="gs-body">
              <div className="gs-title">{s.title}</div>
              <div className="muted gs-desc">{s.body}</div>
            </div>
            {!s.done && (
              <a className="btn sm" href={s.href}>
                {s.cta}
              </a>
            )}
          </li>
        ))}
      </ol>
    </section>
  );
}
