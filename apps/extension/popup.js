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
