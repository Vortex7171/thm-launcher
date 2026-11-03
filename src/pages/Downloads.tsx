import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, errText } from "../api";
import { useApp } from "../App";
import { useDialog } from "../Dialog";

export default function Downloads() {
  const { progress, selected, refresh } = useApp();
  const dlg = useDialog();
  const [u, setU] = useState<any>(null); const [msg, setMsg] = useState("");
  const check = async () => { if (!selected) return; try { setMsg("Checking…"); setU(await api.checkUpdates(selected.id)); setMsg(""); } catch (e) { setMsg(errText(e)); } };
  return (
    <div>
      <header className="head"><div><h1>DOWNLOADS</h1><p className="muted">Progress and updates. Nothing is changed until you confirm.</p></div></header>
      <div className="card"><h3>Active</h3>
        {Object.values(progress).length === 0 && <p className="muted">No downloads yet.</p>}
        {Object.values(progress).map(p => <div key={p.id + p.label}><div className="row between"><span>{p.label}</span><small className="muted">{p.done}/{p.total}</small></div><div className="bar"><div style={{ width: `${p.total ? (p.done / p.total) * 100 : 0}%` }} /></div></div>)}
      </div>
      <div className="card"><h3>Launcher</h3><button className="btn" onClick={() => window.dispatchEvent(new Event("thm-check-update"))}>Check for launcher update</button><p className="muted">Updates are signed and installed in-app. Your accounts and instances are kept.</p></div>
      <div className="card"><h3>Updates for {selected?.name}</h3><button className="btn" onClick={check}>Check for updates</button> <span className="muted">{msg}</span>
        {u && <div>
          <p>Minecraft: {u.minecraft.current} (latest release {u.minecraft.latest_release}) {u.minecraft.newer && <span className="muted">— version changes are manual: clone this instance, then create a new one.</span>}</p>
          {u.fabric && <p>Fabric loader: {u.fabric.current ?? "not installed"} → {u.fabric.latest} {u.fabric.available && <button className="btn" onClick={async () => { if (await dlg.confirm("Update Fabric loader?", "It will be installed on next Play. Your mods and configs are not touched.", "Update")) { await api.applyLoaderUpdate(selected!.id); await refresh(); setMsg("Queued for next Play."); } }}>Update loader</button>}</p>}
          <p>Mods: {u.mods.length ? u.mods.map((m: any) => `${m.file} → ${m.new_version}`).join("; ") + " (apply in Mods)" : "all up to date"}</p>
        </div>}
      </div>
    </div>
  );
}
