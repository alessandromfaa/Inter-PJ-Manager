import { useRef, useState, type FormEvent } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "../components/button";
import { api, ApiError, brl, type Identity } from "../services/api";
import { pagamentoDaQuinzena, preverQuinzena, salarioNaData, type LancamentoQuinzena } from "../services/quinzena";

function centavos(value: string): bigint {
  if (!/^\d+(,\d{1,2})?$/.test(value)) throw new Error("Informe valores como 1500,00.");
  const [inteiro, decimal = ""] = value.split(",");
  return BigInt(inteiro) * 100n + BigInt(decimal.padEnd(2, "0"));
}

type Funcionario = { id: string; nome: string; salario_centavos: string };
type Registro = { id: string; funcionario_id: string; tipo: LancamentoQuinzena["tipo"]; data: string; valor_centavos: string | null; cancelado: boolean };
type Salario = { funcionario_id: string; vigente_a_partir: string; salario_centavos: string };

export function FolhaMock({ user }: { user: Identity }) {
  const cache = useQueryClient();
  const tentativaFuncionario = useRef<{ dados: string; chave: string } | null>(null);
  const tentativaLancamento = useRef<{ dados: string; chave: string } | null>(null);
  const tentativasCancelamento = useRef(new Map<string, string>());
  const tentativaSalario = useRef<{ dados: string; chave: string } | null>(null);
  const [funcionarioId, setFuncionarioId] = useState("");
  const funcionarios = useQuery({ queryKey: ["folha-funcionarios", user.company_id], queryFn: () => api<Funcionario[]>("mock/folha/funcionarios"), enabled: user.roles.includes("ADMIN") });
  const registros = useQuery({ queryKey: ["folha-lancamentos", user.company_id, funcionarioId], queryFn: () => api<Registro[]>(`mock/folha/lancamentos?funcionario_id=${encodeURIComponent(funcionarioId)}`), enabled: !!funcionarioId && user.roles.includes("ADMIN") });
  const salarios = useQuery({ queryKey: ["folha-salarios", user.company_id, funcionarioId], queryFn: () => api<Salario[]>(`mock/folha/salarios?funcionario_id=${encodeURIComponent(funcionarioId)}`), enabled: !!funcionarioId && user.roles.includes("ADMIN") });
  const criarFuncionario = useMutation({ mutationFn: (body: { nome: string; salario_centavos: string; idempotencia: string }) => api<Funcionario>("mock/folha/funcionarios", body), onSuccess: async (funcionario) => { setFuncionarioId(funcionario.id); await cache.invalidateQueries({ queryKey: ["folha-funcionarios", user.company_id] }); } });
  const criarRegistro = useMutation({ mutationFn: (body: { funcionario_id: string; tipo: string; data: string; valor_centavos: string | null; idempotencia: string }) => api<Registro>("mock/folha/lancamentos", body), onSuccess: async () => { await cache.invalidateQueries({ queryKey: ["folha-lancamentos", user.company_id, funcionarioId] }); } });
  const cancelarRegistro = useMutation({ mutationFn: ({ id, chave }: { id: string; chave: string }) => api<void>(`mock/folha/lancamentos/${id}/cancelar`, { idempotencia: chave }, { allowEmpty: true }), onSuccess: async (_, { id }) => { tentativasCancelamento.current.delete(id); setErro(""); await cache.invalidateQueries({ queryKey: ["folha-lancamentos", user.company_id, funcionarioId] }); }, onError: () => setErro("Não foi possível cancelar. Tente novamente para recuperar a mesma solicitação.") });
  const alterarSalario = useMutation({ mutationFn: (body: { funcionario_id: string; vigente_a_partir: string; salario_centavos: string; idempotencia: string }) => api<Salario>("mock/folha/salarios", body), onSuccess: async () => { await cache.invalidateQueries({ queryKey: ["folha-salarios", user.company_id, funcionarioId] }); } });
  const [salario, setSalario] = useState("3000,00");
  const [pagamento, setPagamento] = useState("");
  const [lancamentos, setLancamentos] = useState<LancamentoQuinzena[]>([]);
  const [erro, setErro] = useState("");
  const selecionado = funcionarios.data?.find((item) => item.id === funcionarioId);
  const valores = selecionado ? (registros.data ?? []).filter((item) => !item.cancelado).map((item) => ({ tipo: item.tipo, data: item.data, valorCentavos: item.valor_centavos === null ? undefined : BigInt(item.valor_centavos) })) : lancamentos;
  let previa: ReturnType<typeof preverQuinzena> | undefined;
  let erroPrevia = "";
  if (pagamento && (!selecionado || (registros.isSuccess && salarios.isSuccess))) {
    try { previa = preverQuinzena(selecionado ? salarioNaData(selecionado.salario_centavos, pagamento, salarios.data ?? []) : centavos(salario), pagamento, valores); }
    catch (e) { erroPrevia = e instanceof Error ? e.message : "Não foi possível calcular."; }
  }
  function adicionar(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    const dados = new FormData(form);
    const tipo = String(dados.get("tipo")) as LancamentoQuinzena["tipo"];
    const data = String(dados.get("data"));
    try {
      pagamentoDaQuinzena(data);
      const valorCentavos = tipo === "PRODUCAO" || tipo === "ADIANTAMENTO" ? centavos(String(dados.get("valor"))) : undefined;
      if (valorCentavos !== undefined && valorCentavos <= 0n) throw new Error("Informe valor maior que zero.");
      if (selecionado) {
        const payload = { funcionario_id: selecionado.id, tipo, data, valor_centavos: valorCentavos?.toString() ?? null };
        const fingerprint = JSON.stringify(payload);
        if (tentativaLancamento.current?.dados !== fingerprint) tentativaLancamento.current = { dados: fingerprint, chave: crypto.randomUUID() };
        criarRegistro.mutate({ ...payload, idempotencia: tentativaLancamento.current.chave }, { onSuccess: () => { tentativaLancamento.current = null; form.reset(); }, onError: (error) => setErro(error instanceof ApiError && error.status === 409 ? "Já existe uma falta ativa nesta data ou a solicitação entrou em conflito." : "Não foi possível salvar o lançamento fictício. Repita sem alterar os dados para recuperar a mesma solicitação.") });
      } else {
        if (tipo.startsWith("FALTA_") && lancamentos.some((item) => item.data === data && item.tipo.startsWith("FALTA_"))) throw new Error("Já existe uma falta nesta data.");
        setLancamentos((anteriores) => [...anteriores, { tipo, data, valorCentavos }]);
        form.reset();
      }
      setErro("");
    } catch (e) { setErro(e instanceof Error ? e.message : "Confira os dados."); }
  }
  function cadastrar(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    const dados = new FormData(form);
    try {
      const nome = String(dados.get("nome")).trim();
      const valor = centavos(String(dados.get("salario")));
      if (!nome || valor <= 0n) throw new Error("Informe nome e salário fictícios válidos.");
      const payload = { nome, salario_centavos: valor.toString() };
      const fingerprint = JSON.stringify(payload);
      if (tentativaFuncionario.current?.dados !== fingerprint) tentativaFuncionario.current = { dados: fingerprint, chave: crypto.randomUUID() };
      criarFuncionario.mutate({ ...payload, idempotencia: tentativaFuncionario.current.chave }, { onSuccess: () => { tentativaFuncionario.current = null; form.reset(); setErro(""); }, onError: () => setErro("Não foi possível cadastrar. Repita sem alterar os dados para recuperar a mesma solicitação.") });
    } catch (e) { setErro(e instanceof Error ? e.message : "Confira os dados."); }
  }
  function agendarSalario(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selecionado) return;
    const form = event.currentTarget;
    const dados = new FormData(form);
    try {
      const valor = centavos(String(dados.get("salario")));
      const vigente = String(dados.get("vigente"));
      if (valor <= 0n || pagamentoDaQuinzena(vigente) !== vigente) throw new Error("Informe salário positivo e uma data de pagamento no dia 05 ou 20.");
      const payload = { funcionario_id: selecionado.id, vigente_a_partir: vigente, salario_centavos: valor.toString() };
      const fingerprint = JSON.stringify(payload);
      if (tentativaSalario.current?.dados !== fingerprint) tentativaSalario.current = { dados: fingerprint, chave: crypto.randomUUID() };
      alterarSalario.mutate({ ...payload, idempotencia: tentativaSalario.current.chave }, { onSuccess: () => { tentativaSalario.current = null; form.reset(); setErro(""); }, onError: (error) => setErro(error instanceof ApiError && error.status === 409 ? "Use uma data futura sem alteração salarial cadastrada." : "Não foi possível salvar. Repita sem alterar os dados para recuperar a solicitação.") });
    } catch (e) { setErro(e instanceof Error ? e.message : "Confira os dados."); }
  }
  if (!user.roles.includes("ADMIN")) return <p role="alert">Somente administradores podem acessar a folha MOCK.</p>;
  return <>
    <section className="panel"><span className="badge">MOCK</span><p>Use somente dados fictícios. Cadastros selecionados e seus lançamentos são salvos na base de teste; nenhum pagamento é enviado.</p></section>
    <section className="panel"><h2>Funcionário fictício</h2><form className="form-grid" onSubmit={cadastrar}>
      <label>Nome fictício<input name="nome" maxLength={120} required /></label><label>Salário mensal fictício (R$)<input name="salario" inputMode="decimal" required /></label><Button disabled={criarFuncionario.isPending}>Cadastrar funcionário fictício</Button>
    </form><label>Funcionário<select value={funcionarioId} disabled={criarRegistro.isPending || criarFuncionario.isPending || cancelarRegistro.isPending || alterarSalario.isPending} onChange={(event) => setFuncionarioId(event.target.value)}><option value="">Simulação temporária</option>{funcionarios.data?.map((item) => <option value={item.id} key={item.id}>{item.nome}</option>)}</select></label>{funcionarios.isError && <p role="alert">Não foi possível carregar os funcionários fictícios.</p>}{registros.isError && <p role="alert">Não foi possível carregar os lançamentos fictícios.</p>}{salarios.isError && <p role="alert">Não foi possível carregar o histórico salarial.</p>}</section>
    {selecionado && <section className="panel"><h2>Histórico salarial fictício</h2><p>Salário inicial: {brl(selecionado.salario_centavos)}. Alterações valem a partir de um pagamento futuro no dia 05 ou 20.</p><form className="form-grid" onSubmit={agendarSalario}><label>Novo salário (R$)<input name="salario" inputMode="decimal" required /></label><label>Pagamento a partir de<input name="vigente" type="date" required /></label><Button disabled={alterarSalario.isPending}>Agendar salário fictício</Button></form><ul>{salarios.data?.map((item) => <li key={item.vigente_a_partir}>{item.vigente_a_partir} · {brl(item.salario_centavos)}</li>)}</ul></section>}
    <section className="panel"><h2>Prévia de pagamento</h2><div className="form-grid">
      {!selecionado && <label>Salário da simulação (R$)<input value={salario} onChange={(e) => setSalario(e.target.value)} inputMode="decimal" /></label>}
      <label>Data do pagamento<input type="date" value={pagamento} onChange={(e) => setPagamento(e.target.value)} /></label>
    </div><p>Informe uma data no dia 05 ou 20. O salário é dividido entre as duas datas; o centavo restante fica no dia 05.</p>
    {selecionado && (registros.isPending || salarios.isPending) && <p>Carregando lançamentos e salários…</p>}
    {erroPrevia && <p role="alert">{erroPrevia}</p>}
    {previa && <dl className="forecast"><div><dt>Salário da quinzena</dt><dd>{brl(previa.base.toString())}</dd></div><div><dt>Produção extra</dt><dd>{brl(previa.producao.toString())}</dd></div><div><dt>Faltas injustificadas</dt><dd>{brl(previa.faltas.toString())}</dd></div><div><dt>Adiantamentos</dt><dd>{brl(previa.adiantamentos.toString())}</dd></div><div><dt>Total previsto</dt><dd>{brl(previa.total.toString())}</dd></div></dl>}
    </section>
    <section className="panel"><h2>Lançamento fictício</h2><form className="form-grid" onSubmit={adicionar}>
      <label>Tipo<select name="tipo"><option value="FALTA_INJUSTIFICADA">Falta injustificada</option><option value="FALTA_JUSTIFICADA">Falta justificada</option><option value="ADIANTAMENTO">Adiantamento</option><option value="PRODUCAO">Produção extra</option></select></label>
      <label>Data<input name="data" type="date" required /></label>
      <label>Valor em reais para produção ou adiantamento<input name="valor" inputMode="decimal" placeholder="0,00" /></label>
      <Button disabled={criarRegistro.isPending}>{selecionado ? "Salvar lançamento fictício" : "Adicionar à simulação"}</Button>
    </form>{erro && <p role="alert">{erro}</p>}
    {selecionado ? <ul>{registros.data?.map((item) => <li key={item.id}>{item.data} · {item.tipo.replaceAll("_", " ")} · pagamento {pagamentoDaQuinzena(item.data)}{item.valor_centavos !== null ? ` · ${brl(item.valor_centavos)}` : ""} {item.cancelado ? "· Cancelado" : <Button variant="outline" disabled={cancelarRegistro.isPending} onClick={() => { const chave = tentativasCancelamento.current.get(item.id) ?? crypto.randomUUID(); tentativasCancelamento.current.set(item.id, chave); cancelarRegistro.mutate({ id: item.id, chave }); }}>Cancelar lançamento</Button>}</li>)}</ul> : valores.length > 0 && <ul>{valores.map((item, index) => <li key={index}>{item.data} · {item.tipo.replaceAll("_", " ")} · pagamento {pagamentoDaQuinzena(item.data)}{item.valorCentavos !== undefined ? ` · ${brl(item.valorCentavos.toString())}` : ""} <Button variant="outline" onClick={() => setLancamentos((rows) => rows.filter((_, i) => i !== index))}>Remover</Button></li>)}</ul>}
    </section>
  </>;
}
