// Shared full-page loading state for authenticated app pages. Replaces the
// ad-hoc `<div className="auth-wrap"><p>Loading…</p></div>` scattered across
// pages (which reused the auth-page wrapper and told screen readers nothing).
// Announced via role="status" + aria-busy.
export function PageLoading({
  label = "Loading…",
  wrap = true,
}: {
  label?: string;
  // When false, skip the `.dash` wrapper — for use inside a layout that already
  // provides it (e.g. the settings hub), so padding isn't applied twice.
  wrap?: boolean;
}) {
  const inner = (
    <div className="page-loading" role="status" aria-busy="true">
      <span className="page-loading-spinner" aria-hidden="true" />
      <span className="muted">{label}</span>
    </div>
  );
  return wrap ? <div className="dash">{inner}</div> : inner;
}
