// Popup: show portfolio health, usage and projects for the workspace the saved
// lp_live_ key belongs to. Read-only; every number comes from an MCP tool call.

function show(id) {
  document.getElementById(id).classList.remove("hidden");
}
function hide(id) {
  document.getElementById(id).classList.add("hidden");
}
function openOptions(e) {
  if (e) e.preventDefault();
  chrome.runtime.openOptionsPage();
}

document.getElementById("open-options").addEventListener("click", openOptions);
document.getElementById("to-options").addEventListener("click", openOptions);

function fmtLimit(used, limit) {
  return limit == null ? `${used}` : `${used} / ${limit}`;
}

function fmtPrice(cents, currency) {
  if (cents == null) return "price n/a";
  return `${(cents / 100).toLocaleString()} ${currency || "EUR"}`;
}

async function renderThisPage() {
  const { domain, url } = await lpActiveTab();
  const domEl = document.getElementById("page-domain");
  const mktEl = document.getElementById("page-market");
  const btn = document.getElementById("monitor-btn");
  if (!domain) {
    domEl.textContent = "Not a web page";
    mktEl.textContent = "";
    return;
  }
  domEl.textContent = domain;

  // Marketplace availability (live, free).
  try {
    const hits = await lpTool("lookup_marketplace", { domain });
    if (Array.isArray(hits) && hits.length) {
      const best = hits
        .filter((h) => h.price_cents != null)
        .sort((a, b) => a.price_cents - b.price_cents)[0];
      mktEl.innerHTML = best
        ? `Buyable: <strong>${fmtPrice(best.price_cents, best.currency)}</strong> on ${best.source}${hits.length > 1 ? ` (+${hits.length - 1} more)` : ""}`
        : `Listed on ${hits.map((h) => h.source).join(", ")}`;
    } else {
      mktEl.textContent = "Not found in your marketplaces.";
    }
  } catch (e) {
    mktEl.textContent = e.message;
  }

  // Monitor-this-link needs a default project (set in options).
  const { lpProject = "" } = await chrome.storage.sync.get(["lpProject"]);
  if (lpProject) {
    btn.classList.remove("hidden");
    btn.addEventListener("click", () => void monitorThisPage(lpProject, url));
  } else {
    document.getElementById("monitor-status").textContent =
      "Pick a default project in settings to monitor from here.";
  }
}

async function monitorThisPage(projectId, url) {
  const st = document.getElementById("monitor-status");
  const btn = document.getElementById("monitor-btn");
  btn.disabled = true;
  st.textContent = "Adding…";
  try {
    await lpTool("monitor_url", { project_id: projectId, source_url: url });
    st.textContent = "Added to monitoring. Set the exact link page in LinkPulz if needed.";
  } catch (e) {
    st.textContent = e.message;
    btn.disabled = false;
  }
}

async function render() {
  const { key } = await lpGetConfig();
  if (!key) {
    show("needs-key");
    return;
  }

  // Portfolio health first — it also validates the key.
  let health;
  try {
    health = await lpTool("portfolio_health");
  } catch (e) {
    const el = document.getElementById("error");
    el.textContent = e.message;
    show("error");
    return;
  }
  show("content");
  if (health) {
    document.getElementById("live").textContent = health.live ?? "–";
    document.getElementById("atrisk").textContent = health.at_risk ?? "–";
  }

  // This page: is the current tab's domain buyable in your marketplaces, and
  // can we start monitoring a link on it?
  void renderThisPage();

  // Usage + projects in parallel; each degrades independently.
  const usage = await lpTool("usage").catch(() => null);
  if (usage) {
    document.getElementById("plan").textContent = `Plan: ${usage.plan ?? "—"}`;
    const rows = [
      ["Discoveries", usage.discoveries],
      ["GEO queries", usage.geo_queries],
    ]
      .filter(([, v]) => v)
      .map(([k, v]) => `<li><span>${k}</span><span>${fmtLimit(v.used, v.limit)}</span></li>`)
      .join("");
    document.getElementById("usage").innerHTML = rows || '<li class="muted">No usage yet</li>';
  }

  const projects = await lpTool("list_projects").catch(() => null);
  const ul = document.getElementById("projects");
  if (Array.isArray(projects) && projects.length) {
    ul.innerHTML = projects
      .slice(0, 8)
      .map(
        (p) =>
          `<li><span>${(p.name || p.domain || "project").toString().slice(0, 28)}</span><span class="muted">${(p.domain || "").toString().slice(0, 22)}</span></li>`,
      )
      .join("");
  } else {
    ul.innerHTML = '<li class="muted">No projects</li>';
  }
}

render();
