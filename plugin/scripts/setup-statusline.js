// setup-statusline.js — repoint Claude Code's statusLine at the cc-petline wrapper,
// so the Charm statusline still renders (via ccstatusline) AND the pet gets live
// context%/cost. Reversible: a backup is written and the original command saved.
//
// Usage:  node setup-statusline.js "<abs path to cc-petline.exe>"
//         node setup-statusline.js --revert
//
// JSON-patch via node (not Edit/heredoc) — the proven-reliable method on this box.

const fs = require("fs");
const os = require("os");
const path = require("path");

const settingsPath = path.join(os.homedir(), ".claude", "settings.json");
const markerPath = path.join(os.homedir(), ".cc-petline", "statusline-prev.json");

function load() {
  const raw = fs.readFileSync(settingsPath, "utf8");
  return JSON.parse(raw);
}
function save(obj) {
  fs.copyFileSync(settingsPath, settingsPath + ".bak-cc-petline");
  fs.writeFileSync(settingsPath, JSON.stringify(obj, null, 2));
}

const arg = process.argv[2];

if (arg === "--revert") {
  if (!fs.existsSync(markerPath)) {
    console.error("no saved previous statusLine; nothing to revert");
    process.exit(1);
  }
  const prev = JSON.parse(fs.readFileSync(markerPath, "utf8"));
  const s = load();
  s.statusLine = prev;
  save(s);
  console.log("reverted statusLine to previous value");
  process.exit(0);
}

const exe = arg;
if (!exe) {
  console.error('usage: node setup-statusline.js "<path to cc-petline.exe>"');
  process.exit(1);
}

const s = load();

// Remember the prior statusLine so --revert can restore it.
fs.mkdirSync(path.dirname(markerPath), { recursive: true });
fs.writeFileSync(markerPath, JSON.stringify(s.statusLine || null, null, 2));

s.statusLine = {
  type: "command",
  command: `"${exe}" statusline`,
  padding: 0,
  // 1s is the minimum (refreshInterval is in SECONDS) and the fastest the
  // statusline re-runs while idle — needed so the pet actually animates
  // (it picks a frame per invocation). Higher values look near-frozen.
  refreshInterval: 1,
};
save(s);

console.log("statusLine -> cc-petline wrapper:");
console.log("  " + s.statusLine.command);
console.log("  refreshInterval: 1s (max idle animation rate)");
console.log("backup: settings.json.bak-cc-petline ; revert: node setup-statusline.js --revert");
