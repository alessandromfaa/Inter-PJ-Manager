import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, brl, type Identity } from "../services/api";

type Authorization = {
  id: string;
  nome_pagador: string;
  objeto: string;
  contrato: string;
  periodicidade: string;
  data_inicial: string;
  data_final: string | null;
  valor_centavos: string;
  status: string;
  solicitacao_id: string;
};
type Charge = {
  id: string;
  authorization_id: string;
  competencia: string;
  valor_centavos: string;
  status: string;
  end_to_end_id: string | null;
};

export function PixAutomatico({ user }: { user: Identity }) {
  const cache = useQueryClient();
  const base = "mock/pix-automatico";
  const [message, setMessage] = useState("");
  const [payer, setPayer] = useState("");
  const [object, setObject] = useState("");
  const [contract, setContract] = useState("");
  const [period, setPeriod] = useState("MENSAL");
  const [start, setStart] = useState(new Date().toISOString().slice(0, 10));
  const [end, setEnd] = useState("");
  const [amount, setAmount] = useState("");
  const [selected, setSelected] = useState("");
  const [competence, setCompetence] = useState(new Date().toISOString().slice(0, 10));
  const [chargeAmount, setChargeAmount] = useState("");
  const auths = useQuery({
    queryKey: [base, "autorizações", user.company_id],
    queryFn: () => api<Authorization[]>(base + "/recorrencias"),
  });
  const charges = useQuery({
    queryKey: [base, "cobranças", user.company_id],
    queryFn: () => api<Charge[]>(base + "/cobrancas"),
  });
  const refresh = async () => {
    await Promise.all([
      cache.invalidateQueries({ queryKey: [base, "autorizações", user.company_id] }),
      cache.invalidateQueries({ queryKey: [base, "cobranças", user.company_id] }),
    ]);
  };
  const create = useMutation({
    mutationFn: () => api<Authorization>(base + "/recorrencias", {
      nome_pagador: payer.trim(), objeto: object.trim(), contrato: contract.trim(),
      periodicidade: period, data_inicial: start, data_final: end || null,
      valor_centavos: toCents(amount),
    }),
    onSuccess: async () => { setMessage("Solicitação fictícia criada; aguardando consentimento."); setPayer(""); setObject(""); setContract(""); setAmount(""); await refresh(); },
    onError: () => setMessage("Não foi possível criar a solicitação."),
  });
  const decide = useMutation({
    mutationFn: (input: { id: string; aceitar: boolean }) => api<Authorization>(base + "/recorrencias/decidir", input),
    onSuccess: async () => { setMessage("Decisão simulada em MOCK."); await refresh(); },
    onError: () => setMessage("A autorização não está mais aguardando decisão."),
  });
  const cancel = useMutation({
    mutationFn: (id: string) => api<Authorization>(base + "/recorrencias/cancelar", { id }),
    onSuccess: async () => { setMessage("Autorização fictícia cancelada."); await refresh(); },
    onError: () => setMessage("Não foi possível cancelar a autorização."),
  });
  const issue = useMutation({
    mutationFn: () => api<Charge>(base + "/cobrancas", {
      authorization_id: selected, competencia: competence, valor_centavos: toCents(chargeAmount),
    }),
    onSuccess: async () => { setMessage("Cobrança recorrente fictícia criada."); await refresh(); },
    onError: () => setMessage("Falha: confira autorização ativa, valor e competência ainda não utilizada."),
  });
  const settle = useMutation({
    mutationFn: (id: string) => api<Charge>(base + "/cobrancas/liquidar", { id }),
    onSuccess: async () => { setMessage("Liquidação simulada em MOCK; nenhum pagamento foi enviado."); await refresh(); },
    onError: () => setMessage("Cobrança indisponível para liquidação simulada."),
  });
  const selectedAuth = auths.data?.find((a) => a.id === selected);

  return <div className="stack pix-automatico-page">
    <section className="panel">
      <span className="badge">MOCK · SEM CONEXÃO BANCÁRIA</span>
      <h2>Pix Automático</h2>
      <p>Fluxo demonstrativo de autorização, consentimento, cobrança recorrente e liquidação simulada. Use somente nomes fictícios.</p>
      <form className="form-grid pix-create-form" onSubmit={(e) => { e.preventDefault(); create.mutate(); }}>
        <label>Nome fictício do pagador<input required maxLength={120} value={payer} onChange={(e) => setPayer(e.target.value)} /></label>
        <label>Descrição do vínculo<input required maxLength={35} value={object} onChange={(e) => setObject(e.target.value)} placeholder="Ex.: assinatura mensal" /></label>
        <label>Referência de contrato<input required maxLength={35} value={contract} onChange={(e) => setContract(e.target.value)} /></label>
        <label>Periodicidade<select value={period} onChange={(e) => setPeriod(e.target.value)}>{["SEMANAL", "MENSAL", "TRIMESTRAL", "SEMESTRAL", "ANUAL"].map((p) => <option key={p}>{p}</option>)}</select></label>
        <label>Início<input type="date" required value={start} onChange={(e) => setStart(e.target.value)} /></label>
        <label>Fim (opcional)<input type="date" min={start} value={end} onChange={(e) => setEnd(e.target.value)} /></label>
        <label>Valor recorrente (R$)<input required inputMode="decimal" value={amount} onChange={(e) => setAmount(e.target.value)} placeholder="99,90" /></label>
        <div className="form-actions"><button className="primary form-submit" type="submit" disabled={create.isPending}>Criar solicitação fictícia</button></div>
      </form>
      {message && <p role="status">{message}</p>}
    </section>
    <section className="panel"><h2>Autorizações e consentimento</h2>
      {auths.isPending ? "Carregando…" : auths.error ? <p role="alert">Falha ao carregar autorizações.</p> : auths.data?.length ? auths.data.map((a) => <article className="event authorization-card" key={a.id}>
        <div className="authorization-details"><strong>{a.nome_pagador} · {a.status}</strong><p>{a.objeto} · {a.periodicidade} · {brl(a.valor_centavos)} · início {a.data_inicial}{a.data_final ? ` · fim ${a.data_final}` : ""}</p>
        <small>Solicitação de consentimento fictícia: {a.solicitacao_id}</small></div>
        {a.status === "AGUARDANDO_CONSENTIMENTO" && <div className="actions consent-actions" role="group" aria-label="Decisão do consentimento"><button type="button" className="action-button action-accept" disabled={decide.isPending} onClick={() => decide.mutate({ id: a.id, aceitar: true })}>Simular aceite do pagador</button><button type="button" className="action-button action-reject" disabled={decide.isPending} onClick={() => decide.mutate({ id: a.id, aceitar: false })}>Simular recusa</button></div>}
        {(a.status === "AGUARDANDO_CONSENTIMENTO" || a.status === "ATIVA") && <button type="button" className="action-button action-cancel" disabled={cancel.isPending} onClick={() => cancel.mutate(a.id)}>Cancelar autorização</button>}
      </article>) : <p>Nenhuma solicitação criada.</p>}
    </section>
    <section className="panel"><h2>Cobranças recorrentes de demonstração</h2>
      <form className="form-grid" onSubmit={(e) => { e.preventDefault(); issue.mutate(); }}>
        <label>Autorização ativa<select required value={selected} onChange={(e) => { setSelected(e.target.value); const a = auths.data?.find((x) => x.id === e.target.value); if (a) setChargeAmount((Number(a.valor_centavos) / 100).toFixed(2).replace(".", ",")); }}><option value="">Selecione</option>{auths.data?.filter((a) => a.status === "ATIVA").map((a) => <option key={a.id} value={a.id}>{a.nome_pagador} · {a.periodicidade}</option>)}</select></label>
        <label>Competência<input type="date" required value={competence} onChange={(e) => setCompetence(e.target.value)} /></label>
        <label>Valor da cobrança (R$)<input required inputMode="decimal" value={chargeAmount} onChange={(e) => setChargeAmount(e.target.value)} /></label>
        <div className="form-actions"><button className="primary form-submit" type="submit" disabled={issue.isPending || !selectedAuth}>Criar cobrança fictícia</button></div>
      </form>
      {charges.isPending ? "Carregando…" : charges.error ? <p role="alert">Falha ao carregar cobranças.</p> : charges.data?.length ? charges.data.map((c) => <article className="event" key={c.id}>
        <strong>{c.competencia} · {brl(c.valor_centavos)} · {c.status}</strong>
        {c.end_to_end_id && <p>Identificador de demonstração: {c.end_to_end_id}</p>}
        {c.status === "CRIADA" && <button type="button" className="action-button action-accept" disabled={settle.isPending} onClick={() => settle.mutate(c.id)}>Simular liquidação</button>}
      </article>) : <p>Nenhuma cobrança fictícia criada.</p>}
    </section>
  </div>;
}

function toCents(value: string): string {
  const normalized = value.trim().replace(",", ".");
  if (!/^\d{1,10}(\.\d{1,2})?$/.test(normalized)) throw new Error("Valor inválido");
  const [whole, fraction = ""] = normalized.split(".");
  const cents = BigInt(whole) * 100n + BigInt(fraction.padEnd(2, "0"));
  if (cents < 1n || cents > 999999999999n) throw new Error("Valor fora da faixa");
  return cents.toString();
}
