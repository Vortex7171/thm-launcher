import { useEffect, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, errText } from "../api";
import { useApp } from "../App";
import { useDialog } from "../Dialog";

export default function Accounts() {
  const { accounts, refresh } = useApp();
  const dlg = useDialog();
  const [name, setName] = useState(""); const [msg, setMsg] = useState("");
  const [dc, setDc] = useState<{ user_code: string; verification_uri: string } | null>(null);
  const timer = useRef<number>();
  useEffect(() => () => clearTimeout(timer.current), []);
  const run = async (f: () => Promise<unknown>) => { try { await f(); setMsg(""); await refresh(); } catch (e) { setMsg(errText(e)); } };

  async function msLogin() {
    setMsg("");
    try {
      const d = await api.msBegin(); setDc(d); openUrl(d.verification_uri);
      const end = Date.now() + d.expires_in * 1000;
      const tick = async () => {
        if (Date.now() > end) { setDc(null); setMsg("Login code expired."); return; }
        try { const a = await api.msPoll(d.device_code); if (a) { setDc(null); await refresh(); setMsg(`Signed in as ${a.username}`); return; } }
        catch (e) { setDc(null); setMsg(errText(e)); return; }
        timer.current = window.setTimeout(tick, Math.max(d.interval, 5) * 1000);
      };
      timer.current = window.setTimeout(tick, Math.max(d.interval, 5) * 1000);
    } catch (e) { setMsg(errText(e)); }
  }
  return (
    <div>
      <header className="head"><div><h1>ACCOUNTS</h1><p className="muted">Sign in with Microsoft in two clicks. Tokens are kept in your OS credential vault.</p></div></header>
      {msg && <div className="banner">{msg}</div>}
      <div className="grid2">
        <div className="card"><h3>Microsoft (premium)</h3>
          {dc ? <><p>Enter this code at <b>{dc.verification_uri}</b>:</p><div className="code">{dc.user_code}</div><p className="muted">Waiting for you to finish signing in…</p></>
            : <button className="btn primary" onClick={msLogin}>Sign in with Microsoft</button>}</div>
        <div className="card"><h3>Offline profile</h3>
          <p className="muted">Only works on servers that allow offline-mode logins. This is not a premium account.</p>
          <div className="form"><input placeholder="Username (1–16)" maxLength={16} value={name} onChange={e => setName(e.target.value)} />
            <button className="btn" onClick={() => run(async () => { await api.addOffline(name); setName(""); })}>Add</button></div></div>
      </div>
      <div className="card"><h3>Saved accounts</h3>
        {accounts.length === 0 && <p className="muted">No accounts yet.</p>}
        {accounts.map(a => <div key={a.id} className="slot"><div><b>{a.username}</b> <span className={"tag " + a.kind}>{a.kind === "offline" ? "OFFLINE" : "PREMIUM"}</span> {a.active && <span className="tag on">ACTIVE</span>}</div>
          <div className="actions">{!a.active && <button className="btn" onClick={() => run(() => api.setActive(a.id))}>Use</button>}
            <button className="ghost danger" onClick={async () => { if (await dlg.confirm(`Remove ${a.username}?`, undefined, "Remove", true)) run(() => api.removeAccount(a.id)); }}>Remove</button></div></div>)}
      </div>
    </div>
  );
}
