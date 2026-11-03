// Run once: npm run setup-updater
import fs from "fs"; import os from "os"; import path from "path"; import readline from "readline/promises";
const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
const repo = (await rl.question("Your GitHub repo (format: username/repo-name): ")).trim();
rl.close();
if (!/^[\w.-]+\/[\w.-]+$/.test(repo)) { console.log("That doesn't look like username/repo-name."); process.exit(1); }
const keyPath = path.join(os.homedir(), ".tauri", "thm-launcher.key.pub");
if (!fs.existsSync(keyPath)) { console.log("Public key not found at " + keyPath + "\nDo the 'make a signing key' step first."); process.exit(1); }
const p = "src-tauri/tauri.conf.json";
const c = JSON.parse(fs.readFileSync(p, "utf8"));
c.plugins = { updater: { pubkey: fs.readFileSync(keyPath, "utf8").trim(), endpoints: [`https://github.com/${repo}/releases/latest/download/latest.json`], windows: { installMode: "passive" } } };
fs.writeFileSync(p, JSON.stringify(c, null, 2));
console.log("Done. Updater is now pointed at github.com/" + repo);
