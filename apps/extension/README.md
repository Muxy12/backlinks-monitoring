# LinkPulz Chrome extension

A buildless (Manifest V3, plain JS) companion that shows your LinkPulz
portfolio, usage and projects at a glance. It authenticates with a workspace
`lp_live_` API key and talks to LinkPulz's **own MCP server** (`/api/mcp`) — the
extension is just another agent. The key is stored only in `chrome.storage`
and never leaves your browser.

## Install (unpacked, for now)

1. In LinkPulz, go to **Settings → API keys** and create a key (read-only scopes
   are enough: `read:portfolio` covers the popup).
2. Open `chrome://extensions`, enable **Developer mode**, click **Load
   unpacked**, and select this `apps/extension` folder.
3. Open the extension's **options** (or click *Settings* in the popup), paste the
   key, and **Save & test**. The base URL defaults to `https://linkpulz.com`.

## What it shows

- **This page**: the active tab's domain, whether it's buyable across your
  marketplaces and at what price (`lookup_marketplace`), and a one-click
  **"Monitor this link"** (`monitor_url`) once you pick a default project.
- **Portfolio**: live vs at-risk monitored links (`portfolio_health`).
- **Usage**: plan + discoveries / GEO queries this period (`usage`).
- **Projects**: your target sites (`list_projects`).

All via MCP `tools/call`. Only `monitor_url` writes (adds a monitored backlink);
it needs a key with the **`write:portfolio`** scope and a default project set in
options. Everything else is read-only.

## Roadmap

- An optional on-page badge (content script) instead of the popup "This page"
  card, for users who want it always visible.
- Packaging for the Chrome Web Store once the flows settle.

## Notes

- No build step, no bundler — edit the files and reload the extension.
- Host access is limited to `https://linkpulz.com/*` (set in `manifest.json`);
  change the base URL in options to point at a staging host if needed (add its
  origin to `host_permissions` first).
