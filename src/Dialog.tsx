import { createContext, useContext, useState, type ReactNode } from "react";

type Choice = { label: string; value: string; danger?: boolean };
type Req = { title: string; message?: string; input?: string | null; choices: Choice[]; resolve: (v: any) => void };
type DialogApi = {
  confirm: (title: string, message?: string, okLabel?: string, danger?: boolean) => Promise<boolean>;
  prompt: (title: string, def?: string) => Promise<string | null>;
  choose: (title: string, message: string, choices: Choice[]) => Promise<string | null>;
};
const D = createContext<DialogApi>(null as any);
export const useDialog = () => useContext(D);

export function DialogProvider({ children }: { children: ReactNode }) {
  const [req, setReq] = useState<Req | null>(null);
  const [text, setText] = useState("");
  const open = (r: Omit<Req, "resolve">) => new Promise<any>(res => { setText(r.input ?? ""); setReq({ ...r, resolve: res }); });
  const close = (v: any) => { req?.resolve(v); setReq(null); };
  const api: DialogApi = {
    confirm: (title, message, okLabel = "OK", danger = false) => open({ title, message, choices: [{ label: okLabel, value: "ok", danger }] }).then(v => v === "ok"),
    prompt: (title, def = "") => open({ title, input: def, choices: [{ label: "OK", value: "ok" }] }),
    choose: (title, message, choices) => open({ title, message, choices }),
  };
  return (
    <D.Provider value={api}>
      {children}
      {req && (
        <div className="modal-bg" onClick={() => close(null)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>{req.title}</h3>
            {req.message && <p className="muted">{req.message}</p>}
            {req.input != null && <input autoFocus value={text} onChange={e => setText(e.target.value)} onKeyDown={e => e.key === "Enter" && close(text.trim() || null)} />}
            <div className="actions end">
              <button className="ghost" onClick={() => close(null)}>Cancel</button>
              {req.choices.map(c => (
                <button key={c.value} className={"btn" + (c.danger ? " dangerbtn" : " primary")} onClick={() => close(req.input != null ? (text.trim() || null) : c.value)}>{c.label}</button>
              ))}
            </div>
          </div>
        </div>
      )}
    </D.Provider>
  );
}
