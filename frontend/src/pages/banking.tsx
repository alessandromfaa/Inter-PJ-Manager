import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api, brl, type LiveSnapshot, type Reading } from "../services/api";
import { Button } from "../components/button";
const day = (value: string) => value.split("-").reverse().join("/");
const moment = (value: string) =>
  new Date(value).toLocaleString("pt-BR", { timeZone: "America/Fortaleza" });
type Statement = {
  entries: {
    transaction_id: string;
    date: string;
    included_on: string | null;
    kind: string;
    operation: "C" | "D";
    title: string | null;
    description: string | null;
    document_number: string | null;
    cents: string;
  }[];
  has_more: boolean;
  summary: {
    total_entries: number;
    incoming_cents: string;
    outgoing_cents: string;
    net_cents: string;
  };
};
const operationLabel = (operation: "C" | "D") =>
  operation === "C" ? "Entrada" : "Saída";

function StatementList({
  reading,
  scope,
}: {
  reading: Reading;
  scope: string;
}) {
  const [page, setPage] = useState(0);
  const monthStart = new Date(reading.imported_until + "T12:00:00Z");
  monthStart.setUTCDate(monthStart.getUTCDate() - 29);
  const [begin, setBegin] = useState(
    [reading.imported_from, monthStart.toISOString().slice(0, 10)].sort()[1],
  );
  const [end, setEnd] = useState(reading.imported_until);
  const valid =
    begin >= reading.imported_from &&
    end <= reading.imported_until &&
    begin <= end &&
    Date.parse(end) - Date.parse(begin) < 90 * 86400000;
  const result = useQuery({
    queryKey: [
      "extrato-inter",
      scope,
      begin,
      end,
      page,
      reading.synchronized_at,
    ],
    enabled: valid,
    queryFn: () =>
      api<Statement>(
        "extrato?" +
          new URLSearchParams({
            inicio: begin,
            fim: end,
            pagina: String(page),
          }),
      ),
  });
  return (
    <section className="panel">
      <h2>Lançamentos bancários</h2>
      <p>
        Período importado de {day(reading.imported_from)} a{" "}
        {day(reading.imported_until)}.
      </p>
      <div className="filters">
        <label>
          De{" "}
          <input
            type="date"
            value={begin}
            min={reading.imported_from}
            max={end}
            onChange={(e) => {
              setBegin(e.target.value);
              setPage(0);
            }}
          />
        </label>
        <label>
          Até{" "}
          <input
            type="date"
            value={end}
            min={begin}
            max={reading.imported_until}
            onChange={(e) => {
              setEnd(e.target.value);
              setPage(0);
            }}
          />
        </label>
      </div>
      {!valid ? (
        <p role="alert">Selecione até 90 dias dentro do período importado.</p>
      ) : result.isPending ? (
        <p>Carregando lançamentos…</p>
      ) : result.error ? (
        <p role="alert">{result.error.message}</p>
      ) : (
        <>
          <div className="statement-summary" aria-label="Resumo do período">
            <div>
              <span>Movimentações</span>
              <strong>{result.data.summary.total_entries}</strong>
            </div>
            <div className="positive">
              <span>Entradas</span>
              <strong>{brl(result.data.summary.incoming_cents)}</strong>
            </div>
            <div className="negative">
              <span>Saídas</span>
              <strong>{brl(result.data.summary.outgoing_cents)}</strong>
            </div>
            <div>
              <span>Resultado líquido</span>
              <strong>{brl(result.data.summary.net_cents)}</strong>
            </div>
          </div>
          <div className="scroll">
            <table>
              <thead>
                <tr>
                  <th>Data</th>
                  <th>Movimentação</th>
                  <th>Descrição</th>
                  <th>Documento</th>
                  <th>Valor</th>
                </tr>
              </thead>
              <tbody>
                {result.data.entries.map((entry, i) => (
                  <tr key={entry.transaction_id || i}>
                    <td>{day(entry.date)}</td>
                    <td>
                      <span className={`statement-operation ${entry.operation === "C" ? "is-credit" : "is-debit"}`}>
                        {operationLabel(entry.operation)}
                      </span>
                      <small>{entry.kind.replaceAll("_", " ")}</small>
                    </td>
                    <td>
                      <strong>{entry.title || entry.description || "Sem descrição"}</strong>
                      {entry.title && entry.description && <small>{entry.description}</small>}
                    </td>
                    <td>{entry.document_number || "—"}</td>
                    <td className={entry.operation === "C" ? "money-positive" : "money-negative"}>
                      {entry.operation === "C" ? "+" : "−"}{brl(entry.cents).replace(/^-/, "")}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {!result.data.entries.length && (
            <p>Nenhuma movimentação nesse período.</p>
          )}
          <div className="filters">
            <Button
              variant="outline"
              disabled={page === 0}
              onClick={() => setPage(page - 1)}
            >
              Anterior
            </Button>
            <span>Página {page + 1}</span>
            <Button
              variant="outline"
              disabled={!result.data.has_more}
              onClick={() => setPage(page + 1)}
            >
              Próxima
            </Button>
          </div>
        </>
      )}
    </section>
  );
}
export function Banking({
  data,
  admin,
  scope,
  ledger,
}: {
  data: LiveSnapshot;
  admin: boolean;
  scope: string;
  ledger: boolean;
}) {
  const cache = useQueryClient();
  const sync = useMutation({
    mutationFn: () => api<void>("sincronizar", {}, { allowEmpty: true }),
    onSettled: () => cache.invalidateQueries({ queryKey: ["painel"] }),
  });
  const reading = data.reading;
  const expiry = new Date(data.certificate_expires_at).getTime() - Date.now();
  return (
    <>
      <section className="panel">
        <div className="panelhead">
          <div>
            <h2>Conexão com o Inter</h2>
            <p>
              {reading
                ? "Última sincronização em " + moment(reading.synchronized_at)
                : "Aguardando a primeira sincronização."}
            </p>
          </div>
          {admin && (
            <Button disabled={sync.isPending} onClick={() => sync.mutate()}>
              {sync.isPending ? "Sincronizando…" : "Sincronizar leitura"}
            </Button>
          )}
        </div>
        <p className="syncnote">
          Certificado válido até {moment(data.certificate_expires_at)}.
        </p>
        {expiry < 30 * 86400000 && (
          <p role="alert">
            {expiry <= 0
              ? "Certificado vencido."
              : "O certificado vence em menos de 30 dias."}{" "}
            Atualize os arquivos da integração.
          </p>
        )}
        {(sync.error || data.last_error) && (
          <p role="alert">
            {sync.error?.message ||
              "Última sincronização falhou. Código: " + data.last_error}
          </p>
        )}
      </section>
      {reading &&
        (ledger ? (
          <StatementList
            key={scope + reading.synchronized_at}
            reading={reading}
            scope={scope}
          />
        ) : (
          <>
            <section className="panel">
              <h2>Saldo disponível</h2>
              <p className="stat">
                <strong>{brl(reading.balance.available)}</strong>
              </p>
              {reading.balance.reference_date && (
                <p>
                  Referência do banco em {day(reading.balance.reference_date)}
                </p>
              )}
              <dl className="forecast">
                {(
                  [
                    ["Bloqueado em cheque", reading.balance.blocked_check],
                    ["Bloqueio judicial", reading.balance.blocked_court],
                    ["Bloqueio administrativo", reading.balance.blocked_admin],
                    ["Limite de crédito", reading.balance.credit_limit],
                  ] as const
                ).map(([label, value]) => (
                  <div key={label}>
                    <dt>{label}</dt>
                    <dd>{value === null ? "Não informado" : brl(value)}</dd>
                  </div>
                ))}
              </dl>
            </section>
            <p>Extrato importado até {day(reading.imported_until)}.</p>
          </>
        ))}
    </>
  );
}
