// Shared full-page loading state for authenticated app pages. Replaces the
// ad-hoc `<div className="auth-wrap"><p>Loading…</p></div>` scattered across
// pages (which reused the auth-page wrapper and told screen readers nothing).
// Announced via role="status" + aria-busy.
export function PageLoading({ label = "Loading…" }: { label?: string }) {
  return (
    <div className="dash">
      <div className="page-loading" role="status" aria-busy="true">
        <span className="page-loading-spinner" aria-hidden="true" />
        <span className="muted">{label}</span>
      </div>
    </div>
  );
}
