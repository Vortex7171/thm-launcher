import { createContext, useCallback, useContext, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, errText, LOGO, type Account, type Instance, type Progress } from "./api";
import Home from "./pages/Home"; import Play from "./pages/Play"; import Instances from "./pages/Instances"; import Mods from "./pages/Mods";
import Accounts from "./pages/Accounts"; import Downloads from "./pages/Downloads"; import Settings from "./pages/Settings";
import UpdateBanner from "./UpdateBanner";
import "./styles.css";

const NAV = ["Home", "Play", "Instances", "Mods", "Accounts", "Downloads", "Settings"] as const;
type Page = (typeof NAV)[number];

export type Ctx = {
  instances: Instance[]; accounts: Account[]; selected?: Instance; select: (id: string) => void; refresh: () => Promise<void>;
  progress: Record<string, Progress>; logs: string[]; running: string[]; message: string; busy: boolean;
  crash?: { report?: string | null; code?: number | null }; play: () => Promise<void>; stop: () => Promise<void>; go: (p: Page) => void;
};
const C = createContext<Ctx>(null as any);
export const useApp = () => useContext(C);

export default function App() {
  const [page, setPage] = useState<Page>("Home");
  const [instances, setInstances] = useState<Instance[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [sel, setSel] = useState(localStorage.getItem("sel") ?? "");
  const [progress, setProgress] = useState<Record<string, Progress>>({});
  const [logs, setLogs] = useState<string[]>([]);
  const [running, setRunning] = useState<string[]>([]);
  const [message, setMessage] = useState(""); const [busy, setBusy] = useState(false);
  const [crash, setCrash] = useState<Ctx["crash"]>();
  const selRef = useRef(sel); selRef.current = sel;

  const refresh = useCallback(async () => {
    const [i, a, r] = await Promise.all([api.instances(), api.accounts(), api.running()]);
    setInstances(i); setAccounts(a); setRunning(r);
  }, []);
  useEffect(() => { refresh(); }, [refresh]);
  useEffect(() => {
    const subs = [
      listen<Progress>("progress", e => setProgress(p => ({ ...p, [e.payload.id]: e.payload }))),
      listen<{ line: string }>("game-log", e => setLogs(l => [...l.slice(-1500), e.payload.line])),
      listen("game-started", () => { setCrash(undefined); refresh(); }),
      listen<any>("game-exit", e => { refresh(); setMessage(e.payload.crashed ? "Game crashed." : "Game closed."); if (e.payload.crashed) setCrash({ report: e.payload.report, code: e.payload.code }); }),
    ];
    return () => { subs.forEach(s => s.then(f => f())); };
  }, [refresh]);

  const selected = instances.find(i => i.id === sel) ?? instances[0];
  const select = (id: string) => { setSel(id); localStorage.setItem("sel", id); };

  async function play() {
    if (!selected || busy) return;
    setBusy(true); setCrash(undefined);
    try {
      if (!selected.installed) { setMessage("Installing Minecraft, Fabric, Java and assets…"); await api.install(selected.id); await refresh(); }
      setMessage("Launching…"); await api.launch(selected.id); setMessage("Game running."); setLogs([]); setPage("Play");
    } catch (e) { setMessage(errText(e)); } finally { setBusy(false); await refresh(); }
  }
  async function stop() { if (selected) { await api.kill(selected.id); setMessage("Stopping…"); } }

  const ctx: Ctx = { instances, accounts, selected, select, refresh, progress, logs, running, message, busy, crash, play, stop, go: setPage };
  return (
    <C.Provider value={ctx}>
      <div className="app">
        <aside className="sidebar">
          <div className="brand"><img src={LOGO} alt="" onError={e => ((e.target as HTMLImageElement).style.display = "none")} /><div>THM<span>LAUNCHER</span></div></div>
          {NAV.map(p => <button key={p} className={p === page ? "nav active" : "nav"} onClick={() => setPage(p)}>{p}{p === "Downloads" && Object.values(progress).some(x => x.done < x.total) ? " •" : ""}</button>)}
          <div className="sidebar-foot">All roads lead to THM.</div>
        </aside>
        <main className="main">
          <UpdateBanner />
          {page === "Home" && <Home />}{page === "Play" && <Play />}{page === "Instances" && <Instances />}{page === "Mods" && <Mods />}
          {page === "Accounts" && <Accounts />}{page === "Downloads" && <Downloads />}{page === "Settings" && <Settings />}
        </main>
      </div>
    </C.Provider>
  );
}
