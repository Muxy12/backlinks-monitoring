"use client";

import { useCallback, useEffect, useState, type CSSProperties } from "react";
import { useRouter } from "next/navigation";
import {
  api,
  orgApi,
  type AttentionItemView,
  type MeResponse,
  type OverviewResponse,
} from "@/lib/api";
import { selectedOrg, selectedOrgId } from "@/lib/workspace";
import { stageLabel } from "@/lib/campaigns";
import { money, moneyList, num, pct, timeAgo } from "@/lib/format";
import { GrowthChart } from "@/components/GrowthChart";
import { Donut, type DonutSegment } from "@/components/Donut";
import { GettingStarted } from "./GettingStarted";
import { Tour, TOUR_KEY } from "./Tour";
import { Icon } from "../icons";
import "@/app/auth.css";
import "./dashboard.css";
import "./overview.css";
import "./onboarding.css";

const RANGES: [string, string][] = [
  ["30d", "30D"],
  ["3m", "3M"],
  ["6m", "6M"],
  ["1y", "1Y"],
];

// Friendly labels for the recent-activity feed.
const EVENT_LABELS: Record<string, string> = {
  backlink_found: "Backlink found",
  backlink_lost: "Backlink lost",
  backlink_restored: "Backlink recovered",
  anchor_changed: "Anchor changed",
  target_changed: "Target changed",
  dofollow_to_nofollow: "Changed to nofollow",
  nofollow_to_dofollow: "Changed to dofollow",
  link_moved: "Link moved",
  source_404: "Source page 404",
  target_404: "Target page 404",
  source_404_to_200: "Source page recovered",
};

export default function OverviewPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [range, setRange] = useState("30d");
  const [data, setData] = useState<OverviewResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [tourRun, setTourRun] = useState(false);

  // First login: start the guided tour once (per browser).
  useEffect(() => {
    if (!orgId) return;
    try {
      if (!localStorage.getItem(TOUR_KEY)) setTourRun(true);
    } catch {
      /* private mode, skip the tour rather than loop */
    }
  }, [orgId]);

  useEffect(() => {
    api.me().then((res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      setMe(res.data);
      setOrgId(selectedOrgId(res.data.organizations));
      setLoading(false);
    });
  }, [router]);

  const load = useCallback(async (org: string, r: string) => {
    const res = await orgApi.overview(org, r);
    if (res.ok) {
      setData(res.data);
      setError(false);
    } else {
      setError(true);
    }
  }, []);

  useEffect(() => {
    if (orgId) void load(orgId, range);
  }, [orgId, range, load]);

  if (loading)
    return (
      <main className="dash" aria-busy="true">
        <div className="tiles">
          {[0, 1, 2, 3].map((i) => (
            <div key={i} className="tile skeleton-rows">
              <div className="sk-row" />
            </div>
          ))}
        </div>
      </main>
    );
  if (!me) return null;

  const beta = !!me.features?.campaigns_beta;
  const org = selectedOrg(me.organizations);

  return (
    <main className="dash">
      <header className="ov-head">
        <div>
          <h1>Overview</h1>
          <p className="ov-sub">
            Your link-building performance at a glance
            {org ? ` · ${org.name}` : ""}
            {data ? ` · updated ${timeAgo(data.generated_at)}` : ""}
          </p>
        </div>
        <div className="ov-actions">
          <div className="range-tabs" role="group" aria-label="Date range">
            {RANGES.map(([slug, lbl]) => (
              <button
                key={slug}
                className={`btn sm${range === slug ? " primary" : ""}`}
                aria-pressed={range === slug}
                onClick={() => setRange(slug)}
              >
                {lbl}
              </button>
            ))}
          </div>
          {beta && (
            <button className="btn sm primary" onClick={() => router.push("/campaigns")}>
              Start Campaign
            </button>
          )}
          <button className="btn sm" onClick={() => router.push("/import")}>
            Import Backlinks
          </button>
        </div>
      </header>

      {orgId && <GettingStarted org={orgId} onStartTour={() => setTourRun(true)} />}

      {!orgId ? (
        <p className="muted">Create a workspace to get started.</p>
      ) : error && !data ? (
        <div className="panel">
          <p className="muted">Couldn’t load the overview.</p>
          <button className="btn sm" onClick={() => void load(orgId, range)}>
            Retry
          </button>
        </div>
      ) : !data ? (
        <div className="tiles">
          {[0, 1, 2, 3].map((i) => (
            <div key={i} className="tile skeleton-rows">
              <div className="sk-row" />
            </div>
          ))}
        </div>
      ) : (
        <OverviewBody data={data} beta={beta} />
      )}

      <Tour run={tourRun} onClose={() => setTourRun(false)} />
    </main>
  );
}

function OverviewBody({ data, beta }: { data: OverviewResponse; beta: boolean }) {
  const router = useRouter();
  const go = (href: string) => router.push(href);
  const s = data.summary;
  const health = s.link_health;

  return (
    <>
      {/* ── 4 KPIs ── */}
      <div className="tiles">
        <KpiCard
          label="Link Health"
          tip="Percentage of monitored backlinks currently healthy."
          value={pct(health.percentage, 0)}
          hint={
            health.eligible > 0
              ? `${num(health.healthy)} / ${num(health.eligible)} healthy`
              : "No monitored links yet"
          }
          delta={health.change}
          accent="var(--live)"
          icon="check"
          onClick={() => go("/backlinks?filter=protected")}
        />
        <KpiCard
          label="Protected Value"
          tip="Acquisition value of backlinks currently protected by LinkPulz."
          value={moneyList(s.protected_value)}
          hint="Investment protected"
          accent="var(--primary)"
          icon="billing"
          onClick={() => go("/backlinks?filter=protected")}
        />
        <KpiCard
          label="Active Campaigns"
          tip="Link-building campaigns currently running."
          value={num(s.active_campaigns.total)}
          hint={`${s.active_campaigns.active} active · ${s.active_campaigns.draft} draft`}
          accent="var(--violet)"
          icon="campaigns"
          onClick={() => go(beta ? "/campaigns" : "/operations")}
        />
        <KpiCard
          label="Value at Risk"
          tip="Acquisition value of protected backlinks currently needing attention."
          value={moneyList(s.value_at_risk)}
          hint={s.value_at_risk.length ? "Needs attention" : "Nothing at risk"}
          accent="var(--lost)"
          icon="warn"
          onClick={() => go("/backlinks?filter=lost")}
        />
      </div>

      {/* ── Needs Your Attention ── */}
      <section className="panel" style={{ marginTop: 16 }}>
        <div className="panel-head">
          <h2>Needs Your Attention</h2>
          {data.attention.length > 0 && <span className="badge">{data.attention.length}</span>}
        </div>

        {data.next_best_action && (
          <div className="nba">
            <div>
              <div className="nba-label">Next best action</div>
              <div className="attn-title">{data.next_best_action.title}</div>
              {data.next_best_action.subtitle && (
                <div className="attn-sub">{data.next_best_action.subtitle}</div>
              )}
            </div>
            <button className="btn sm primary" onClick={() => go(data.next_best_action!.route)}>
              {data.next_best_action.action}
            </button>
          </div>
        )}

        {data.attention.length === 0 ? (
          <div className="ov-caught-up">
            <span className="ok">
              <Icon name="check" size={18} />
            </span>
            <span>You’re all caught up, nothing needs action right now.</span>
          </div>
        ) : (
          <div className="attention-list">
            {data.attention.map((a, i) => (
              <AttentionRow key={i} a={a} onAction={() => go(a.route)} />
            ))}
          </div>
        )}
      </section>

      {/* ── Growth | Campaign performance ── */}
      <div className="ov-grid" style={{ marginTop: 16 }}>
        <section className="panel">
          <div className="panel-head">
            <h2>Backlink Growth</h2>
          </div>
          <GrowthChart series={data.backlink_growth.series} />
          <div className="growth-summary">
            <span className="gs">
              <b style={{ color: "var(--live)" }}>+{num(data.backlink_growth.new_count)}</b>New
            </span>
            <span className="gs">
              <b style={{ color: "var(--lost)" }}>-{num(data.backlink_growth.lost_count)}</b>Lost
            </span>
            <span className="gs">
              <b>
                {data.backlink_growth.net_count >= 0 ? "+" : ""}
                {num(data.backlink_growth.net_count)}
              </b>
              Net
            </span>
          </div>
        </section>

        <section className="panel">
          <div className="panel-head">
            <h2>Campaign Performance</h2>
          </div>
          {data.campaign_performance ? (
            <CampaignPerformance perf={data.campaign_performance} onView={() => go("/campaigns")} />
          ) : (
            <EmptyBlock
              text={
                beta
                  ? "Turn competitor data into an actionable link-building plan."
                  : "Campaigns are coming soon for your workspace."
              }
              cta={beta ? { label: "Start Campaign", href: "/campaigns" } : undefined}
              onCta={go}
            />
          )}
        </section>
      </div>

      {/* ── Top opportunities ── */}
      {beta && (
        <section className="panel" style={{ marginTop: 16 }}>
          <div className="panel-head">
            <h2>Top Opportunities</h2>
            {data.top_opportunities.length > 0 && (
              <button className="btn sm" onClick={() => go("/campaigns")}>
                View all
              </button>
            )}
          </div>
          {data.top_opportunities.length === 0 ? (
            <EmptyBlock
              text="No opportunities yet. Seed prospects from the competitor gap."
              cta={{ label: "Open campaigns", href: "/campaigns" }}
              onCta={go}
            />
          ) : (
            <div className="bl-scroll">
              <table className="bl-table">
                <thead>
                  <tr>
                    <th>Domain</th>
                    <th>Score</th>
                    <th>Why</th>
                    <th>Est. cost</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  {data.top_opportunities.map((o) => (
                    <tr key={o.prospect_id}>
                      <td className="mono">{o.domain}</td>
                      <td>{o.score ?? "-"}</td>
                      <td className="muted">{o.score_reason ?? "-"}</td>
                      <td>{o.estimated_cost ? money(o.estimated_cost, o.currency) : "-"}</td>
                      <td style={{ textAlign: "right" }}>
                        <button className="btn sm" onClick={() => go(`/campaigns/${o.campaign_id}`)}>
                          Review
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>
      )}

      {/* ── Competitive position ── */}
      {data.competitive_position && (
        <section className="panel" style={{ marginTop: 16 }}>
          <div className="panel-head">
            <h2>Competitive Position</h2>
            <button className="btn sm" onClick={() => go("/competitors")}>
              Explore
            </button>
          </div>
          <div className="comp-grid">
            <span className="ch">Referring domains, you</span>
            <span className="cv">{num(data.competitive_position.our_referring_domains)}</span>
            <span className="cv" />
            <span className="ch">Referring domains, competitors</span>
            <span className="cv">
              {num(data.competitive_position.competitor_referring_domains)}
            </span>
            <span className="cv" />
          </div>
          {data.competitive_position.biggest_gap_domain && (
            <p className="muted" style={{ marginTop: 10 }}>
              Biggest opportunity:{" "}
              <span className="mono">{data.competitive_position.biggest_gap_domain}</span> links to{" "}
              {data.competitive_position.biggest_gap_competitors} of your competitors but not you.
            </p>
          )}
        </section>
      )}

      {/* ── Post-Link Impact signal (§40) ── */}
      {data.post_link && (
        <section className="panel" style={{ marginTop: 16 }}>
          <div className="panel-head">
            <h2>
              Post-Link Impact{" "}
              <span
                style={{
                  fontSize: 9,
                  fontWeight: 700,
                  letterSpacing: "0.08em",
                  padding: "2px 6px",
                  borderRadius: 999,
                  background: "var(--grad-soft)",
                  color: "var(--violet)",
                  verticalAlign: "middle",
                }}
              >
                BETA
              </span>
            </h2>
          </div>
          <p className="muted" style={{ marginTop: 0 }}>
            {data.post_link.tracked} link{data.post_link.tracked === 1 ? "" : "s"} tracked
            {data.post_link.measured > 0 ? ` · ${data.post_link.measured} with 28-day data` : ""}
            {data.post_link.collecting > 0 ? ` · ${data.post_link.collecting} still collecting` : ""}
            .
          </p>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
            {(
              [
                ["var(--live)", data.post_link.positive, "positive"],
                ["var(--changed)", data.post_link.neutral, "no change"],
                ["var(--lost)", data.post_link.negative, "negative"],
                ["var(--pending)", data.post_link.collecting, "collecting"],
              ] as const
            ).map(([color, n, label]) => (
              <span
                key={label}
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 6,
                  fontSize: 12,
                  padding: "3px 9px",
                  borderRadius: 999,
                  border: "1px solid var(--border)",
                }}
              >
                <span
                  style={{ width: 7, height: 7, borderRadius: "50%", background: color }}
                />
                {n} {label}
              </span>
            ))}
          </div>
          <p
            className="muted"
            style={{ fontSize: 11.5, fontStyle: "italic", margin: "10px 0 0" }}
          >
            Observed response after acquisition, correlation, not causation.
          </p>
        </section>
      )}

      {/* ── Recent activity ── */}
      <section className="panel" style={{ marginTop: 16 }}>
        <div className="panel-head">
          <h2>Recent Activity</h2>
        </div>
        {data.recent_activity.length === 0 ? (
          <p className="muted">No recent activity.</p>
        ) : (
          <div className="activity-list">
            {data.recent_activity.map((e, i) => (
              <div key={i} className="act-item">
                <span>{EVENT_LABELS[e.event_type] ?? e.event_type}</span>
                <span className="mono muted">{e.source_domain}</span>
                <span className="act-when">{timeAgo(e.detected_at)}</span>
              </div>
            ))}
          </div>
        )}
      </section>
    </>
  );
}

function KpiCard({
  label,
  tip,
  value,
  hint,
  delta,
  accent,
  icon,
  onClick,
}: {
  label: string;
  tip: string;
  value: string;
  hint: string;
  delta?: number | null;
  accent?: string;
  icon?: string;
  onClick: () => void;
}) {
  const dir = delta == null ? null : delta > 0 ? "up" : delta < 0 ? "down" : "flat";
  const style = accent ? ({ "--tile-accent": accent } as CSSProperties) : undefined;
  return (
    <button className="tile kpi-link kpi-accent" onClick={onClick} title={tip} style={style}>
      <div className="tile-top">
        <span className="tile-label">{label}</span>
        {icon && (
          <span className="tile-icon">
            <Icon name={icon} size={16} />
          </span>
        )}
      </div>
      <div className="tile-value" style={accent ? { color: accent } : undefined}>
        {value}
      </div>
      <div className="tile-foot">
        {dir && (
          <span className={`delta-pill ${dir}`}>
            {dir === "up" ? "↑" : dir === "down" ? "↓" : "→"} {Math.abs(delta!).toFixed(1)}%
          </span>
        )}
        <span className="tile-hint">{hint}</span>
      </div>
    </button>
  );
}

function AttentionRow({ a, onAction }: { a: AttentionItemView; onAction: () => void }) {
  return (
    <div className="attn-item">
      <span className={`attn-dot ${a.priority}`} title={`${a.priority} priority`}>
        <span className="sr-only">{a.priority} priority</span>
      </span>
      <div className="attn-main">
        <div className="attn-title">{a.title}</div>
        {a.subtitle && <div className="attn-sub">{a.subtitle}</div>}
      </div>
      {a.value_at_risk && (
        <span className="attn-value">
          {money(a.value_at_risk.amount, a.value_at_risk.currency)}
        </span>
      )}
      <button className="btn sm" onClick={onAction}>
        {a.action}
      </button>
    </div>
  );
}

function CampaignPerformance({
  perf,
  onView,
}: {
  perf: NonNullable<OverviewResponse["campaign_performance"]>;
  onView: () => void;
}) {
  // Budget vs spend bar for the dominant shared currency (no FX).
  const bCur = perf.budget[0];
  const sForCur = bCur ? perf.spend.find((x) => x.currency === bCur.currency) : perf.spend[0];
  const budgetN = bCur ? parseFloat(bCur.amount) : 0;
  const spendN = sForCur ? parseFloat(sForCur.amount) : 0;
  const fill = budgetN > 0 ? Math.min(100, (spendN / budgetN) * 100) : 0;

  const stageVal = (slug: string) =>
    perf.stage_counts.find((x) => x.stage === slug)?.count ?? 0;
  const STAGES: [string, string][] = [
    ["identified", "var(--pending)"],
    ["contacted", "var(--primary)"],
    ["negotiating", "var(--changed)"],
    ["won", "var(--live)"],
  ];
  const segments: DonutSegment[] = STAGES.map(([slug, color]) => ({
    label: stageLabel(slug),
    value: stageVal(slug),
    color,
  }));
  const totalProspects = segments.reduce((a, b) => a + b.value, 0);

  return (
    <>
      {bCur ? (
        <>
          <div style={{ fontSize: 14, fontWeight: 600 }}>
            {money(spendN, bCur.currency)} / {money(budgetN, bCur.currency)}
          </div>
          <div className="budget-bar">
            <div className="budget-fill" style={{ width: `${fill}%` }} />
          </div>
        </>
      ) : (
        <p className="muted" style={{ marginTop: 0 }}>
          No budget set · spend {moneyList(perf.spend)}
        </p>
      )}
      {totalProspects > 0 ? (
        <Donut
          segments={segments}
          centerValue={num(totalProspects)}
          centerLabel={totalProspects === 1 ? "prospect" : "prospects"}
        />
      ) : (
        <div className="funnel">
          {STAGES.map(([st]) => (
            <span className="fstage" key={st}>
              <b>{num(stageVal(st))}</b>
              {stageLabel(st)}
            </span>
          ))}
        </div>
      )}
      <button className="btn sm" style={{ marginTop: 12 }} onClick={onView}>
        View Campaigns
      </button>
    </>
  );
}

function EmptyBlock({
  text,
  cta,
  onCta,
}: {
  text: string;
  cta?: { label: string; href: string };
  onCta: (href: string) => void;
}) {
  return (
    <div className="chart-empty" style={{ flexDirection: "column", gap: 10 }}>
      <span>{text}</span>
      {cta && (
        <button className="btn sm primary" onClick={() => onCta(cta.href)}>
          {cta.label}
        </button>
      )}
    </div>
  );
}
