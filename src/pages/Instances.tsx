import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { api, errText, type Instance } from "../api";
import { useApp } from "../App";
import { useDialog } from "../Dialog";

export default function Instances() {
  const { instances, selected, select, refresh, running } = useApp();
  const dlg = useDialog();
  const [versions, setVersions] = useState<string[]>([]);
  const [form, setForm] = useState({ name: "", mc: "", loader: "fabric" });
  const [edit, setEdit] = useState<Instance | null>(null);
  const [msg, setMsg] = useState("");
  useEffect(() => { api.mcVersions().then(v => { setVersions(v); setForm(f => ({ ...f, mc: f.mc || v[0] })); }).catch(e => setMsg(errText(e))); }, []);
  const run = async (f: () => Promise<unknown>, ok = "") => { try { await f(); setMsg(ok); await refresh(); } catch (e) { setMsg(errText(e)); } };

  return (
    <div>
      <header className="head"><div><h1>INSTANCES</h1><p className="muted">Each instance has its own version, mods, configs, saves and Java settings.</p></div>
        <button className="btn" onClick={async () => { const p = await open({ filters: [{ name: "THM instance", extensions: ["zip"] }] }); if (typeof p === "string") run(() => api.importInstance(p), "Imported.") }}>Import</button></header>
      {msg && <div className="banner">{msg}</div>}
      <div className="card"><h3>New instance</h3>
        <div className="form">
          <input placeholder="Name" value={form.name} onChange={e => setForm({ ...form, name: e.target.value })} />
          <select value={form.mc} onChange={e => setForm({ ...form, mc: e.target.value })}>{versions.map(v => <option key={v}>{v}</option>)}</select>
          <select value={form.loader} onChange={e => setForm({ ...form, loader: e.target.value })}><option value="fabric">Fabric</option><option value="vanilla">Vanilla</option></select>
          <button className="btn primary" onClick={() => run(async () => { const i = await api.createInstance(form.name, form.mc, form.loader); select(i.id); setForm({ ...form, name: "" }); }, "Created.")}>Create</button>
        </div></div>
      {instances.map(i => (
        <div key={i.id} className={"card inst" + (selected?.id === i.id ? " sel" : "")}>
          <div className="row between">
            <div><b>{i.name}</b> {i.kind === "thm" && <span className="tag">THM</span>} {running.includes(i.id) && <span className="tag on">RUNNING</span>}
              <p className="muted">{i.mc_version} · {i.loader}{i.loader_version ? " " + i.loader_version : ""} · {i.installed ? "installed" : "not installed"} · {i.memory_mb} MB</p></div>
            <div className="actions">
              <button className="ghost" onClick={() => select(i.id)}>Select</button>
              <button className="ghost" onClick={() => setEdit(edit?.id === i.id ? null : { ...i })}>Settings</button>
              <button className="ghost" onClick={async () => { const n = await dlg.prompt("Rename instance", i.name); if (n) run(() => api.renameInstance(i.id, n)); }}>Rename</button>
              <button className="ghost" onClick={async () => { const n = await dlg.prompt("Name for the copy", i.name + " copy"); if (n) run(() => api.cloneInstance(i.id, n), "Cloned."); }}>Clone</button>
              <button className="ghost" onClick={() => run(() => api.repairInstance(i.id), "Will re-verify all files on next Play.")}>Repair</button>
              <button className="ghost" onClick={async () => { const p = await save({ defaultPath: `${i.name}.zip`, filters: [{ name: "zip", extensions: ["zip"] }] }); if (p) run(() => api.exportInstance(i.id, p), "Exported."); }}>Export</button>
              <button className="ghost" onClick={() => revealItemInDir(i.game_dir)}>Folder</button>
              <button className="ghost danger" onClick={async () => { const c = await dlg.choose(`Delete "${i.name}"?`, "Remove it from the launcher only, or also permanently erase its mods, worlds and settings.", [{ label: "Remove, keep files", value: "keep" }, { label: "Delete everything", value: "all", danger: true }]); if (c) run(() => api.deleteInstance(i.id, c === "all"), "Deleted."); }}>Delete</button>
            </div>
          </div>
          {edit?.id === i.id && (
            <div className="form wrap">
              <label>Memory (MB)<input type="number" value={edit.memory_mb} onChange={e => setEdit({ ...edit, memory_mb: +e.target.value })} /></label>
              <label>JVM arguments<input value={edit.jvm_args} onChange={e => setEdit({ ...edit, jvm_args: e.target.value })} /></label>
              <label>Java path (blank = auto)<input value={edit.java_path} onChange={e => setEdit({ ...edit, java_path: e.target.value })} /></label>
              <label>Server (quick connect)<input value={edit.server} onChange={e => setEdit({ ...edit, server: e.target.value })} /></label>
              <label>Width<input type="number" value={edit.width} onChange={e => setEdit({ ...edit, width: +e.target.value })} /></label>
              <label>Height<input type="number" value={edit.height} onChange={e => setEdit({ ...edit, height: +e.target.value })} /></label>
              <label className="chk"><input type="checkbox" checked={edit.fullscreen} onChange={e => setEdit({ ...edit, fullscreen: e.target.checked })} /> Fullscreen</label>
              <button className="btn primary" onClick={() => run(async () => { await api.updateInstance(edit); setEdit(null); }, "Saved.")}>Save</button>
            </div>)}
        </div>))}
    </div>
  );
}
