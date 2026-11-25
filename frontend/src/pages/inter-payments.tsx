import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, brl, type Identity } from "../services/api";
import { Button } from "../components/button";
import { MfaPanel } from "./mock-payments";

const base = "pagamentos/pix";
type Entry = {
  id: string;
  chave: string;
  valor_centavos: string;
  situacao: string;
  status_banco: string | null;
  pode_aprovar: boolean;
  pode_enviar: boolean;
  pode_consultar: boolean;
};
type List = { intencoes: Entry[]; envio_habilitado: boolean };
type Action = "aprovar" | "enviar";
const label = (s: string) => s.toLocaleLowerCase("pt-BR").replaceAll("_", " ");

export function InterPayments({ user, onExpired }: { user: Identity; onExpired: () => void }) {
  const cache = useQueryClient();
  const queryKey = ["pagamentos-inter", user.company_id, user.id];
  const list = useQuery({ queryKey, queryFn: () => api<List>(base) });
  const [key, setKey] = useState("");
  const storageKey = "pj:pix:preparar:" + user.company_id + ":" + user.id;
  const [request, setRequest] = useState(() => {
    const existing = sessionStorage.getItem(storageKey);
    if (existing) return existing;
    const id = crypto.randomUUID();
    sessionStorage.setItem(storageKey, id);
    return id;
  });
  const [selection, setSelection] = useState<{ entry: Entry; action: Action }>();
  const [code, setCode] = useState("");
  const [message, setMessage] = useState("");
  const mutation = useMutation({
    mutationFn: ({ path, data }: { path: string; data: unknown }) =>
      api<{ situacao: string }>(path, data),
    retry: false,
    gcTime: 0,
    onSuccess: (reply, input) => {
      if (input.path === base) {
        setKey("");
        const next = crypto.randomUUID();
        sessionStorage.setItem(storageKey, next);
        setRequest(next);
      }
      setSelection(undefined);
      setMessage("Situação da intenção: " + label(reply.situacao) + ".");
    },
    onSettled: () => {
      setCode("");
      void cache.invalidateQueries({ queryKey });
      void cache.invalidateQueries({ queryKey: ["auditoria"] });
    },
  });
  const busy = mutation.isPending;
  return <>
    <MfaPanel user={user} onExpired={onExpired} />
    {user.roles.some(r => r === "ADMIN" || r === "OPERATOR") && <section className="panel">
      <h2>Preparar pagamento por chave</h2>
      <p>Valor do piloto: R$ 0,01. Outra pessoa da empresa deve revisar e aprovar a intenção.</p>
      <form className="form-grid" onSubmit={event => {
        event.preventDefault();
        mutation.mutate({ path: base, data: { chave: key, valor_centavos: "1", idempotencia: request } });
      }}>
        <label>Chave Pix do destinatário<input required maxLength={77} pattern="\S+" autoComplete="off" value={key}
          onChange={event => setKey(event.target.value)} disabled={busy} /></label>
        <Button disabled={busy}>Encaminhar para revisão</Button>
      </form>
    </section>}
    <section className="panel">
      <div className="panelhead"><h2>Intenções de pagamento</h2><Button variant="outline" onClick={() => void list.refetch()}>Atualizar</Button></div>
      {list.isPending ? <p>Carregando intenções…</p> : list.isError ? <p role="alert">{list.error.message}</p> : <>
        {!list.data.envio_habilitado && <p>Preparação e aprovação disponíveis. O envio ao banco está desativado.</p>}
        {list.data.intencoes.length === 0 && <p>Nenhuma intenção cadastrada.</p>}
        <ul>{list.data.intencoes.map(entry => <li key={entry.id} className="event">
          <div><strong>{brl(entry.valor_centavos)} · {entry.chave}</strong><p>{label(entry.status_banco ?? entry.situacao)}</p>
          {["EM_ENVIO","INDETERMINADO"].includes(entry.situacao) && <p>Confira o resultado no Inter antes de preparar outro pagamento.</p>}</div>
          <div className="actions">{(["aprovar", "enviar"] as const).filter(action => action === "aprovar" ? entry.pode_aprovar : entry.pode_enviar).map(action =>
            <Button key={action} disabled={busy} onClick={() => {setSelection({entry,action}); setCode(""); setMessage(""); mutation.reset();}}>
              {action === "aprovar" ? "Revisar para aprovação" : "Revisar para envio"}</Button>)}
            {entry.pode_consultar && <Button disabled={busy} variant="outline" onClick={() => mutation.mutate({path: base + "/consultar", data: {id: entry.id}})}>Consultar no Inter</Button>}
          </div></li>)}</ul>
      </>}
      {selection && <form className="form-grid" onSubmit={event => {
        event.preventDefault();
        const {entry,action}=selection;
        mutation.mutate({path: base + "/" + action, data: {id: entry.id, chave: entry.chave, valor_centavos: entry.valor_centavos, codigo_mfa: code}});
      }}>
        <p>{selection.action === "aprovar" ? "Aprovar a intenção" : "Transferir pelo Inter"} de {brl(selection.entry.valor_centavos)} para {selection.entry.chave}?</p>
        <label>Código do autenticador<input required autoComplete="one-time-code" inputMode="numeric" pattern="[0-9]{6}" maxLength={6} value={code} onChange={e => setCode(e.target.value)} /></label>
        <Button disabled={busy}>{selection.action === "aprovar" ? "Confirmar aprovação" : "Confirmar e enviar Pix"}</Button>
        <Button type="button" disabled={busy} variant="outline" onClick={() => {setSelection(undefined); setCode("");}}>Voltar</Button>
      </form>}
      {mutation.error && <p role="alert">{mutation.error.message}</p>}
      {message && <p role="status">{message}</p>}
    </section>
  </>;
}
