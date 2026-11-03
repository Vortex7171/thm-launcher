// Usage: npm run release -- 0.1.1   (bumps the version everywhere)
import fs from "fs";
const v = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(v || "")) { console.log("Usage: npm run release -- 0.1.1"); process.exit(1); }
const edit = (f, fn) => fs.writeFileSync(f, fn(fs.readFileSync(f, "utf8")));
edit("package.json", s => s.replace(/"version": ".*?"/, `"version": "${v}"`));
edit("src-tauri/tauri.conf.json", s => s.replace(/"version": ".*?"/, `"version": "${v}"`));
edit("src-tauri/Cargo.toml", s => s.replace(/^version = ".*"$/m, `version = "${v}"`));
console.log(`Version set to ${v}. Now run:\n  git add .\n  git commit -m "v${v}"\n  git tag v${v}\n  git push\n  git push --tags`);
