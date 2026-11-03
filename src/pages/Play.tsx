import { useEffect, useRef } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useApp } from "../App";

export default function Play() {
  const { selected, play, stop, busy, running, logs, crash, message } = useApp();
  const ref = useRef<HTMLPreElement>(null);
  useEffect(() => { ref.current?.scrollTo(0, ref.current.scrollHeight); }, [logs]);
  const isRunning = !!selected && running.includes(selected.id);
  return (
    <div>
      <header className="head"><div><h1>PLAY</h1><p className="muted">{selected ? `${selected.name} · ${selected.mc_version}` : "No instance"}</p></div>
        {isRunning ? <button className="btn" onClick={stop}>Stop game</button> : <button className="btn primary" disabled={!selected || busy} onClick={play}>{busy ? "Working…" : "Launch"}</button>}</header>
      {crash && <div className="banner">The game exited abnormally (code {String(crash.code ?? "?")}). {crash.report && <button className="ghost" onClick={() => revealItemInDir(crash.report!)}>Show crash report</button>}</div>}
      <p className="muted">{message}</p>
      <pre className="console" ref={ref}>{logs.length ? logs.join("\n") : "Game output appears here."}</pre>
    </div>
  );
}
