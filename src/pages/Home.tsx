import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, THM_DISCORD, THM_SITE } from "../api";
import { useApp } from "../App";

export default function Home() {
  const { instances, selected, select, accounts, play, stop, busy, running, message, progress, go } = useApp();
  const [status, setStatus] = useState<any>(null);
  const [news, setNews] = useState<{ title: string; body: string; date?: string }[]>([]);
  const [act, setAct] = useState<{ time: string; text: string }[]>([]);
  const acct = accounts.find(a => a.active);
  const server = selected?.server || "play.6b6t.org";
  const isRunning = !!selected && running.includes(selected.id);
  const prog = selected ? progress[selected.id] : undefined;

  useEffect(() => { api.status(server).then(setStatus); }, [server]);
  useEffect(() => { api.announcements().then(setNews).catch(() => setNews([])); api.activity().then(setAct); }, [message]);

  return (
    <div>
      <header className="head">
        <div><h1>THE HIGHWAY MEN</h1><p className="muted">Nether highways on 6b6t</p></div>
        <div className="pill" data-on={!!status?.online}><span className="dot" />{status?.online ? `${status.players?.online ?? "?"} / ${status.players?.max ?? "?"} online` : "Server status unavailable"}</div>
      </header>
      <section className="grid2">
        <div className="card">
          <label>Instance</label>
          <select value={selected?.id ?? ""} onChange={e => select(e.target.value)}>{instances.map(i => <option key={i.id} value={i.id}>{i.name} — {i.mc_version}</option>)}</select>
          <label>Account</label>
          <div className="row">{acct ? <><b>{acct.username}</b><span className={"tag " + acct.kind}>{acct.kind === "offline" ? "OFFLINE" : "PREMIUM"}</span></> : <button className="ghost" onClick={() => go("Accounts")}>Add an account →</button>}</div>
          {isRunning
            ? <button className="play stop" onClick={stop}>STOP</button>
            : <button className="play" disabled={!selected || !acct || busy} onClick={play}>{busy ? "WORKING…" : selected?.installed ? "PLAY" : "INSTALL & PLAY"}</button>}
          {prog && prog.done < prog.total && <div className="prog"><div className="bar"><div style={{ width: `${(prog.done / prog.total) * 100}%` }} /></div><small>{prog.label} · {prog.done}/{prog.total}</small></div>}
          {message && <p className="muted">{message}</p>}
        </div>
        <div className="card">
          <div className="meta"><span>Minecraft</span><b>{selected?.mc_version ?? "—"}</b></div>
          <div className="meta"><span>Loader</span><b>{selected ? `${selected.loader}${selected.loader_version ? " " + selected.loader_version : ""}` : "—"}</b></div>
          <div className="meta"><span>Server</span><b>{server}</b></div>
          <div className="meta"><span>MOTD</span><b className="clip">{status?.motd?.clean?.[0]?.trim() || "—"}</b></div>
          <div className="meta"><span>Memory</span><b>{selected ? `${selected.memory_mb} MB` : "—"}</b></div>
          <div className="links"><button className="ghost" onClick={() => openUrl(THM_SITE)}>highwaymen.cc ↗</button><button className="ghost" onClick={() => openUrl(THM_DISCORD)}>Discord ↗</button></div>
        </div>
      </section>
      <section className="grid2">
        <div className="card"><h3>Announcements</h3>{news.length ? news.slice(0, 4).map((n, i) => <div key={i} className="item"><b>{n.title}</b><p className="muted">{n.body}</p></div>) : <p className="muted">No announcements. Set an announcements feed URL in Settings to show THM news here.</p>}</div>
        <div className="card"><h3>Recent activity</h3>{act.length ? act.slice(0, 6).map((a, i) => <div key={i} className="item"><span>{a.text}</span><small className="muted"> {new Date(a.time).toLocaleString()}</small></div>) : <p className="muted">Nothing yet.</p>}</div>
      </section>
    </div>
  );
}
