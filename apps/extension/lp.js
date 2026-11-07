// Shared helpers: config storage + MCP JSON-RPC client. No build step; plain ES
// loaded by popup.js and options.js. The extension is "an agent" — it talks to
// LinkPulz's own MCP server (/api/mcp) with a workspace lp_live_ key. The key
// never leaves the browser's extension storage.

const DEFAULT_BASE = "https://linkpulz.com";

/** Read {key, base} from sync storage. base defaults to production. */
async function lpGetConfig() {
  const { lpKey = "", lpBase = DEFAULT_BASE } = await chrome.storage.sync.get([
    "lpKey",
    "lpBase",
  ]);
  return { key: lpKey.trim(), base: (lpBase || DEFAULT_BASE).replace(/\/+$/, "") };
}

async function lpSetConfig({ key, base }) {
  await chrome.storage.sync.set({ lpKey: key.trim(), lpBase: base.trim() || DEFAULT_BASE });
}

let RPC_ID = 0;

/** One JSON-RPC call to the MCP endpoint. Throws on HTTP or RPC error. */
async function lpMcp(method, params) {
  const { key, base } = await lpGetConfig();
  if (!key) throw new Error("No API key set. Open the extension options to add one.");
  let resp;
  try {
    resp = await fetch(`${base}/api/mcp`, {
      method: "POST",
      headers: { "content-type": "application/json", authorization: `Bearer ${key}` },
      body: JSON.stringify({ jsonrpc: "2.0", id: ++RPC_ID, method, params }),
    });
  } catch (e) {
    throw new Error(`Could not reach ${base}. Check the base URL and your connection.`);
  }
  if (resp.status === 401) throw new Error("The API key was rejected (401). Check or re-create it.");
  if (!resp.ok) throw new Error(`Request failed (HTTP ${resp.status}).`);
  const body = await resp.json();
  if (body.error) throw new Error(body.error.message || "MCP error");
  return body.result;
}

/** Call a tool and parse its JSON text content. */
async function lpTool(name, args) {
  const result = await lpMcp("tools/call", { name, arguments: args || {} });
  const text = result && result.content && result.content[0] && result.content[0].text;
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}
