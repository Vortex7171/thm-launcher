import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, errText, THM_DISCORD, THM_SITE, type Settings as S } from "../api";

export default function Settings() {
  const [s, setS] = useState<S | null>(null); const [msg, setMsg] = useState(""); const [v, setV] = useState("");
  const [slots, setSlots] = useState<any[]>([]);
  useEffect(() => { api.settings().then(setS); api.version().then(setV); api.instances().then(async i => { if (i[0]) setSlots(await api.slots(i[0].id)); }); }, []);
  if (!s) return null;
  const f = (k: keyof S) => (e: React.ChangeEvent<HTMLInputElement>) => setS({ ...s, [k]: e.target.value });
  const save = async () => { try { await api.saveSettings(s); setMsg("Saved."); } catch (e) { setMsg(errText(e)); } };
  return (
    <div className="settings">
      <header className="head"><div><h1>SETTINGS</h1><p className="muted">THM Launcher {v}</p></div><button className="btn primary" onClick={save}>Save</button></header>
      {msg && <div className="banner">{msg}</div>}
      <div className="card"><h3>Launcher updates</h3>
        <label className="chk"><input type="checkbox" checked={s.auto_update} onChange={e => setS({ ...s, auto_update: e.target.checked })} /> Check for launcher updates when the app starts</label>
        <p className="muted">When off, nothing is checked automatically. You can still check by hand. Updates are never installed without your click.</p>
        <button className="ghost" onClick={() => window.dispatchEvent(new Event("thm-check-update"))}>Check now</button></div>
      <div className="card"><h3>Java & server</h3>
        <label>Global Java path (blank = managed Java, downloaded automatically)<input value={s.java_path} onChange={f("java_path")} /></label>
        <label>Default server<input value={s.default_server} onChange={f("default_server")} /></label>
        <label>Custom Azure client ID (optional — normal Microsoft login needs nothing)<input value={s.ms_client_id} onChange={f("ms_client_id")} /></label></div>
      <div className="card"><h3>Update & news feeds</h3>
        <label>THM manifest URL (JSON, same shape as the bundled one)<input value={s.thm_manifest_url} onChange={f("thm_manifest_url")} /></label>
        <button className="ghost" onClick={async () => { try { await api.saveSettings(s); await api.refreshManifest(); setMsg("Manifest refreshed."); } catch (e) { setMsg(errText(e)); } }}>Refresh manifest now</button>
        <label>Announcements feed URL (JSON array: title, body, date)<input value={s.announcements_url} onChange={f("announcements_url")} /></label></div>
      <div className="card"><h3>THM sources</h3><p className="muted">Direct JAR URLs for components that have no public download. Leave blank to import JARs manually.</p>
        {slots.filter(x => x.source !== "modrinth").map(x => <label key={x.id}>{x.name} URL<input value={s.slot_urls[x.id] ?? ""} onChange={e => setS({ ...s, slot_urls: { ...s.slot_urls, [x.id]: e.target.value } })} /></label>)}</div>
      <div className="card"><h3>The Highway Men</h3><div className="links"><button className="ghost" onClick={() => openUrl(THM_SITE)}>highwaymen.cc ↗</button><button className="ghost" onClick={() => openUrl(THM_DISCORD)}>Join Discord ↗</button></div></div>
    </div>
  );
}
