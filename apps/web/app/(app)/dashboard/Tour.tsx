"use client";

import { useCallback, useEffect, useState, type CSSProperties } from "react";

// A lightweight, dependency-free coachmark tour. It spotlights real elements by
// CSS selector (a dimmed overlay with a cutout ring via box-shadow) and shows a
// tooltip with Next / Back / Skip. No eval, no injected scripts — it works under
// the strict nonce/strict-dynamic CSP. A missing target (e.g. a collapsed mobile
// sidebar) gracefully falls back to a centered tooltip. Seen-state is per-browser
// in localStorage; "Take the tour" on the checklist replays it.

export const TOUR_KEY = "backlio_tour_v1";

type TourStep = { target: string | null; title: string; body: string };

const STEPS: TourStep[] = [
  {
    target: null,
    title: "Welcome to LinkPulz 👋",
    body: "Let's get you monitoring your backlinks in about a minute. Three short steps.",
  },
  {
    target: '[data-tour="checklist"]',
    title: "Your setup checklist",
    body: "Follow these three steps and you're live — it ticks them off automatically as you go.",
  },
  {
    target: 'a.side-link[href="/backlinks"]',
    title: "Add your backlinks",
    body: "This is where you paste or import the links you want to watch. LinkPulz re-checks each one on a schedule and alerts you when something changes.",
  },
  {
    target: 'a.side-link[href="/settings/notifications"]',
    title: "Set up alerts",
    body: "Choose how you hear about changes — email, Slack or webhook. In-app alerts are always on.",
  },
  {
    target: null,
    title: "You're ready 🚀",
    body: "Start by adding your first backlink. You can replay this tour anytime from the checklist.",
  },
];

const TIP_W = 320;
const TIP_EST_H = 180;
const PAD = 8;

export function Tour({ run, onClose }: { run: boolean; onClose: () => void }) {
  const [i, setI] = useState(0);
  const [rect, setRect] = useState<DOMRect | null>(null);

  const step = STEPS[i];

  const measure = useCallback(() => {
    const target = STEPS[i]?.target;
    if (!target) {
      setRect(null);
      return;
    }
    const el = document.querySelector(target);
    if (el) {
      el.scrollIntoView({ block: "center", behavior: "smooth" });
      setRect(el.getBoundingClientRect());
    } else {
      setRect(null);
    }
  }, [i]);

  useEffect(() => {
    if (run) measure();
  }, [run, measure]);

  useEffect(() => {
    if (!run) return;
    const on = () => measure();
    window.addEventListener("resize", on);
    window.addEventListener("scroll", on, true);
    return () => {
      window.removeEventListener("resize", on);
      window.removeEventListener("scroll", on, true);
    };
  }, [run, measure]);

  const finish = useCallback(() => {
    try {
      localStorage.setItem(TOUR_KEY, "1");
    } catch {
      /* private mode */
    }
    setI(0);
    setRect(null);
    onClose();
  }, [onClose]);

  useEffect(() => {
    if (!run) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") finish();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [run, finish]);

  if (!run) return null;

  const next = () => (i < STEPS.length - 1 ? setI(i + 1) : finish());
  const back = () => setI((n) => Math.max(0, n - 1));

  const box = rect
    ? {
        top: rect.top - PAD,
        left: rect.left - PAD,
        width: rect.width + PAD * 2,
        height: rect.height + PAD * 2,
      }
    : null;

  let tipStyle: CSSProperties | undefined;
  if (box) {
    const w = typeof window !== "undefined" ? window.innerWidth : 1200;
    const h = typeof window !== "undefined" ? window.innerHeight : 800;
    let top = box.top + box.height + 12;
    if (top + TIP_EST_H > h) top = Math.max(12, box.top - TIP_EST_H - 12);
    const left = Math.min(Math.max(12, box.left), w - TIP_W - 12);
    tipStyle = { top, left, width: TIP_W };
  }

  return (
    <div className="tour-root" role="dialog" aria-modal="true" aria-label="Product tour">
      {box ? (
        <div
          className="tour-spot"
          style={{ top: box.top, left: box.left, width: box.width, height: box.height }}
        />
      ) : (
        <div className="tour-scrim" />
      )}
      <div className={box ? "tour-tip" : "tour-tip centered"} style={tipStyle}>
        <div className="tour-step-n">
          {i + 1} / {STEPS.length}
        </div>
        <h3>{step.title}</h3>
        <p>{step.body}</p>
        <div className="tour-actions">
          <button className="tour-skip" onClick={finish}>
            Skip
          </button>
          <div className="tour-nav">
            {i > 0 && (
              <button className="btn sm ghost" onClick={back}>
                Back
              </button>
            )}
            <button className="btn sm primary" onClick={next}>
              {i < STEPS.length - 1 ? "Next" : "Done"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
