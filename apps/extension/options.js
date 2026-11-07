// Options: save the API key + base URL, then verify by calling the `usage` tool.

const keyEl = document.getElementById("key");
const baseEl = document.getElementById("base");
const statusEl = document.getElementById("status");

async function load() {
  const { key, base } = await lpGetConfig();
  keyEl.value = key;
  baseEl.value = base;
}

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
  } catch (e) {
    setStatus(e.message, false);
  }
});

load();
