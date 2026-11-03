import { useEffect, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { api, errText } from "./api";

export default function UpdateBanner() {
  const [upd, setUpd] = useState<Update | null>(null);
  const [msg, setMsg] = useState("");
  const [pct, setPct] = useState<number | null>(null);

  const run = async (manual: boolean) => {
    try {
      if (manual) setMsg("Checking for updates…");
      const u = await check();
      setUpd(u);
      setMsg(u ? "" : manual ? "You're on the latest version." : "");
    } catch (e) { if (manual) setMsg("Update check failed: " + errText(e)); }
  };
  useEffect(() => {
    api.settings().then(s => { if (s.auto_update) run(false); }).catch(() => {});
    const h = () => run(true);
    window.addEventListener("thm-check-update", h);
    return () => window.removeEventListener("thm-check-update", h);
  }, []);

  async function install() {
    let total = 0, got = 0;
    try {
      await upd!.downloadAndInstall(ev => {
        if (ev.event === "Started") total = ev.data.contentLength ?? 0;
        if (ev.event === "Progress") { got += ev.data.chunkLength; if (total) setPct(Math.round((got / total) * 100)); }
        if (ev.event === "Finished") setPct(100);
      });
      await relaunch();
    } catch (e) { setMsg(errText(e)); setPct(null); }
  }

  if (!upd && !msg) return null;
  return (
    <div className="banner update">
      <div>
        {upd ? <><b>Update available: v{upd.version}</b>{upd.body && <div className="muted">{upd.body}</div>}</> : msg}
        {upd && msg && <div className="muted">{msg}</div>}
      </div>
      {upd && <button className="btn primary" disabled={pct !== null} onClick={install}>{pct === null ? "Install & restart" : `Downloading ${pct}%`}</button>}
    </div>
  );
}
