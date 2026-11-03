import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, THM_DISCORD, THM_SITE } from "../api";
import { useApp } from "../App";

export default function Home() {
  const { instances, selected, select, accounts, play, stop, busy, running, message, progress, go } = useApp();
  const [status, setStatus] = useState<any>(null);
  const [news, setNews] = useState<{ title: string; body: string }[]>([]);
  const [act, setAct] = useState<{ time: string; text: string }[]>([]);
  const acct = accounts.find(a => a.active);
  const server = selected?.server || "play.6b6t.org";
  const isRunning = !!selected && running.includes(selected.id);
  const prog = selected ? progress[selected.id] : undefined;

  useEffect(() => { api.status(server).then(setStatus); }, [server]);
  useEffect(() => { api.announcements().then(setNews).catch(() => setNews([])); api.activity().then(setAct); }, [message]);

  return (
    <div className="home">
      <section className="hero">
        <div className="road" />
        <div className="hero-top">
          <div className="pill" data-on={!!status?.online}><span className="dot" />{status?.online ? `${status.players?.online ?? "?"} / ${status.players?.max ?? "?"} online` : "Server offline"}</div>
        </div>
        <div className="hero-body">
          <h1>The Highway Men</h1>
          <p className="tagline">Nether highways on 6b6t</p>
          <div className="launchrow">
            <div className="pick"><small>Instance</small>
              <select value={selected?.id ?? ""} onChange={e => select(e.target.value)}>{instances.map(i => <option key={i.id} value={i.id}>{i.name} · {i.mc_version}</option>)}</select></div>
            <div className="pick"><small>Account</small>
              {acct ? <div className="who"><b>{acct.username}</b><span className={"tag " + acct.kind}>{acct.kind === "offline" ? "offline" : "premium"}</span></div> : <div className="who muted">None yet</div>}</div>
            {isRunning ? <button className="play stop" onClick={stop}>STOP</button>
              : !acct ? <button className="play" onClick={() => go("Accounts")}>ADD AN ACCOUNT</button>
              : <button className="play" disabled={!selected || busy} onClick={play}>{busy ? "WORKING…" : selected?.installed ? "PLAY" : "INSTALL & PLAY"}</button>}
          </div>
          {prog && prog.done < prog.total && <div className="prog"><div className="bar"><div style={{ width: `${(prog.done / prog.total) * 100}%` }} /></div><small>{prog.label} · {prog.done}/{prog.total}</small></div>}
          {message && <p className="msg">{message}</p>}
        </div>
      </section>

      <section className="stats">
        <div><small>Minecraft</small><b>{selected?.mc_version ?? "—"}</b></div>
        <div><small>Loader</small><b>{selected ? `${selected.loader[0].toUpperCase()}${selected.loader.slice(1)}` : "—"}</b></div>
        <div><small>Server</small><b>{server}</b></div>
        <div><small>Memory</small><b>{selected ? `${(selected.memory_mb / 1024).toFixed(selected.memory_mb % 1024 ? 1 : 0)} GB` : "—"}</b></div>
      </section>

      <section className="cols">
        <div><h3>Recent activity</h3>{act.length ? act.slice(0, 5).map((a, i) => <div key={i} className="item">{a.text}<small className="muted"> · {new Date(a.time).toLocaleDateString()}</small></div>) : <p className="muted">Nothing yet. Press Play to get started.</p>}</div>
        <div><h3>{news.length ? "Announcements" : "Community"}</h3>
          {news.length ? news.slice(0, 3).map((n, i) => <div key={i} className="item"><b>{n.title}</b><p className="muted">{n.body}</p></div>)
            : <div className="item links"><button className="ghost" onClick={() => openUrl(THM_SITE)}>highwaymen.cc ↗</button><button className="ghost" onClick={() => openUrl(THM_DISCORD)}>Join the Discord ↗</button></div>}</div>
      </section>
    </div>
  );
}
