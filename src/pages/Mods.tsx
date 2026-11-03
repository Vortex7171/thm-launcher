import { useCallback, useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, errText, type Analysis, type Slot } from "../api";
import { useApp } from "../App";
import { useDialog } from "../Dialog";

export default function Mods() {
  const { selected } = useApp();
  const dlg = useDialog();
  const id = selected?.id ?? "";
  const [an, setAn] = useState<Analysis | null>(null);
  const [slots, setSlots] = useState<Slot[]>([]);
  const [q, setQ] = useState(""); const [hits, setHits] = useState<any[]>([]);
  const [ups, setUps] = useState<any[] | null>(null);
  const [msg, setMsg] = useState(""); const [open1, setOpen1] = useState<string>("");
  const load = useCallback(async () => { if (!id) return; try { setAn(await api.analyze(id)); setSlots(await api.slots(id)); } catch (e) { setMsg(errText(e)); } }, [id]);
  useEffect(() => { load(); setUps(null); setHits([]); }, [load]);
  const run = async (f: () => Promise<unknown>, ok = "") => { try { await f(); setMsg(ok); await load(); } catch (e) { setMsg(errText(e)); } };
  if (!selected) return <p className="muted">Create an instance first.</p>;
  const importJar = async () => { const p = await open({ multiple: true, filters: [{ name: "Fabric mod", extensions: ["jar"] }] }); if (p) for (const f of Array.isArray(p) ? p : [p]) await run(() => api.addJar(id, f), "Imported."); };
  const issues = an ? an.duplicates.length + an.missing.length + an.incompatible.length : 0;

  return (
    <div>
      <header className="head"><div><h1>MODS</h1><p className="muted">{selected.name} · {selected.mc_version} · Fabric</p></div>
        <div className="actions"><button className="btn" onClick={importJar}>Import JAR</button>
          <button className="btn" onClick={async () => { const p = await save({ defaultPath: `${selected.name}-mods.zip` }); if (p) run(() => api.exportMods(id, p), "Exported."); }}>Export mods</button></div></header>
      {msg && <div className="banner">{msg}</div>}

      <div className="card"><h3>THM components</h3>
        {slots.map(s => (
          <div key={s.id} className="slot"><div><b>{s.name}</b> <span className={"tag" + (s.installed ? " on" : "")}>{s.installed ? "INSTALLED" : "MISSING"}</span>
            <p className="muted">{s.info}</p></div>
            <div className="actions">
              {s.link && <button className="ghost" onClick={() => openUrl(s.link!)}>Website ↗</button>}
              {!s.installed && s.can_download && <button className="btn" onClick={() => run(() => api.installSlot(id, s.id), `${s.name} installed.`)}>Download</button>}
              {!s.installed && !s.can_download && <button className="btn" onClick={importJar}>Import JAR</button>}
            </div></div>))}
      </div>

      {an && issues > 0 && <div className="card warn"><h3>Problems detected ({issues})</h3>
        {an.duplicates.map(d => <p key={d.id}>Duplicate mod <b>{d.id}</b>: {d.files.join(", ")}</p>)}
        {an.missing.map((m, i) => <p key={i}><b>{m.mod}</b> needs <b>{m.dep}</b>, which is not installed.</p>)}
        {an.incompatible.map((m, i) => <p key={i}><b>{m.mod}</b> may not support Minecraft {m.minecraft} (requires {m.requires}).</p>)}
        <p className="muted">Compatibility checks are best-effort based on each mod's fabric.mod.json.</p></div>}

      <div className="card"><h3>Installed ({an?.mods.length ?? 0})</h3>
        {an?.mods.length === 0 && <p className="muted">No mods yet.</p>}
        {an?.mods.map(m => (
          <div key={m.file} className="mod">
            <div onClick={() => setOpen1(open1 === m.file ? "" : m.file)} className="grow"><b>{m.name ?? m.file}</b> <small className="muted">{m.version} · {(m.size / 1024).toFixed(0)} KB</small>
              {open1 === m.file && <p className="muted">{m.description || "No description."}<br />File: {m.file}<br />Depends: {m.depends.join(", ") || "—"}</p>}</div>
            <label className="switch"><input type="checkbox" checked={m.enabled} onChange={e => run(() => api.toggleMod(id, m.file, e.target.checked))} /><span /></label>
            <button className="ghost danger" onClick={async () => { if (await dlg.confirm("Delete mod?", m.file, "Delete", true)) run(() => api.deleteMod(id, m.file)); }}>Delete</button>
          </div>))}
      </div>

      <div className="card"><h3>Updates</h3>
        <button className="btn" onClick={async () => { try { setUps(await api.modUpdates(id)); } catch (e) { setMsg(errText(e)); } }}>Check for mod updates</button>
        {ups && (ups.length ? ups.map(u => <div key={u.file} className="slot"><span>{u.file}: {u.current} → <b>{u.new_version}</b></span>
          <button className="btn" onClick={() => run(async () => { await api.applyModUpdate(id, u.file, u.url, u.sha1, u.filename); setUps(ups.filter(x => x.file !== u.file)); }, "Updated (old build kept as .bak).")}>Update</button></div>) : <p className="muted">Everything Modrinth knows about is up to date.</p>)}
      </div>

      <div className="card"><h3>Search Modrinth</h3>
        <div className="form"><input placeholder="Search Fabric mods for this version…" value={q} onChange={e => setQ(e.target.value)} onKeyDown={e => e.key === "Enter" && api.mrSearch(id, q).then(setHits).catch(x => setMsg(errText(x)))} />
          <button className="btn" onClick={() => api.mrSearch(id, q).then(setHits).catch(x => setMsg(errText(x)))}>Search</button></div>
        {hits.map(h => <div key={h.project_id} className="slot"><div><b>{h.title}</b><p className="muted">{h.description}</p></div>
          <button className="btn" onClick={() => run(() => api.mrInstall(id, h.project_id), `${h.title} installed.`)}>Install</button></div>)}
      </div>
    </div>
  );
}
