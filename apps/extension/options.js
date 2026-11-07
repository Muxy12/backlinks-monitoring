// Options: save the API key + base URL, then verify by calling the `usage` tool.

const keyEl = document.getElementById("key");
const baseEl = document.getElementById("base");
const projectEl = document.getElementById("project");
const statusEl = document.getElementById("status");

async function loadProjects(selected) {
  try {
    const projects = await lpTool("list_projects");
    if (!Array.isArray(projects)) return;
    projectEl.length = 1; // keep the leading "— none —" option
    for (const p of projects) {
      const opt = document.createElement("option");
      opt.value = p.id;
      opt.textContent = `${p.name || p.domain || p.id}`;
      if (p.id === selected) opt.selected = true;
      projectEl.appendChild(opt);
    }
  } catch {
    /* key not set/valid yet — the dropdown stays "none" */
  }
}

async function load() {
  const { key, base } = await lpGetConfig();
  keyEl.value = key;
  baseEl.value = base;
  const { lpProject = "" } = await chrome.storage.sync.get(["lpProject"]);
  if (key) await loadProjects(lpProject);
}

projectEl.addEventListener("change", () => {
  void chrome.storage.sync.set({ lpProject: projectEl.value });
});

function setStatus(msg, ok) {
  statusEl.textContent = msg;
  statusEl.className = ok ? "ok" : "muted";
}

document.getElementById("save").addEventListener("click", async () => {
  const key = keyEl.value.trim();
  const base = baseEl.value.trim() || "https://linkpulz.com";
  if (!key.startsWith("lp_live_")) {
    setStatus("That doesn't look like an lp_live_ key.", false);
    return;
  }
  await lpSetConfig({ key, base });
  setStatus("Saved. Testing…", false);
  try {
    const usage = await lpTool("usage");
    setStatus(`Connected — plan: ${usage?.plan ?? "ok"}.`, true);
    await loadProjects(projectEl.value); // populate the project picker now
  } catch (e) {
    setStatus(e.message, false);
  }
});

load();
