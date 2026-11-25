import { useState } from "react";
import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { Wallet, ArrowDownLeft, ArrowUpRight, Search } from "lucide-react";
import { type Snapshot, brl, date } from "../services/api";
export function Ledger({
  s,
  full = false,
  open,
}: {
  s: Snapshot;
  full?: boolean;
  open?: () => void;
}) {
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState("");
  const rows = s.entries.filter(
    (e) =>
      e.description
        .toLocaleLowerCase("pt-BR")
        .includes(query.toLocaleLowerCase("pt-BR")) &&
      (!kind || (e.cents > 0 ? "entrada" : "saída") === kind),
  );
  return (
    <section className="panel">
      <div className="panelhead">
        <h2>{full ? "Movimentações" : "Últimas movimentações"}</h2>
        {open && (
          <button className="link" onClick={open}>
            Ver extrato
          </button>
        )}
      </div>
      {full && (
        <div className="filters">
          <label>
            <Search size={16} />
            <input
              aria-label="Buscar movimentação"
              value={query}
              placeholder="Buscar movimentação"
              onChange={(e) => setQuery(e.target.value)}
            />
          </label>
          <select
            aria-label="Tipo de movimentação"
            value={kind}
            onChange={(e) => setKind(e.target.value)}
          >
            <option value="">Todos os tipos</option>
            <option>entrada</option>
            <option>saída</option>
          </select>
        </div>
      )}
      <div className="scroll">
        <table>
          <thead>
            <tr>
              <th>Descrição</th>
              <th>Categoria</th>
              <th>Data</th>
              <th>Valor</th>
            </tr>
          </thead>
          <tbody>
            {(full ? rows : rows.slice(0, 4)).map((e, index) => (
              <tr key={`${e.date}:${e.description}:${index}`}>
                <td>
                  <span className={"trans " + (e.cents > 0 ? "credit" : "")}>
                    {e.cents > 0 ? (
                      <ArrowDownLeft size={16} />
                    ) : (
                      <ArrowUpRight size={16} />
                    )}
                  </span>
                  {e.description}
                </td>
                <td>{e.category}</td>
                <td>{date(e.date)}</td>
                <td className={e.cents > 0 ? "positive" : ""}>
                  {e.cents > 0 ? "+" : ""}
                  {brl(e.cents)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {!rows.length && <p>Nenhuma movimentação encontrada.</p>}
    </section>
  );
}
export function Overview({ s, open }: { s: Snapshot; open: () => void }) {
  let saldo = s.opening;
  const points = [
    { day: date(s.as_of.slice(0, 7) + "-01"), saldo },
    ...s.entries
      .slice()
      .reverse()
      .map((e) => ({ day: date(e.date), saldo: (saldo += e.cents) })),
  ];
  return (
    <>
      <div className="cards">
        {[
          {
            name: "Saldo disponível",
            value: s.available,
            icon: Wallet,
            note: "Conta demonstrativa",
          },
          {
            name: "Entradas no período",
            value: s.incoming,
            icon: ArrowDownLeft,
            note: "Créditos realizados",
          },
          {
            name: "Saídas no período",
            value: s.outgoing,
            icon: ArrowUpRight,
            note: "Débitos realizados",
          },
        ].map(({ name, value, icon: Icon, note }, i) => (
          <article key={name} className={"stat s" + i}>
            <div>
              {name}
              <Icon size={21} />
            </div>
            <strong>{brl(value)}</strong>
            <small>{note}</small>
          </article>
        ))}
      </div>
      <section className="panel">
        <div className="panelhead">
          <div>
            <h2>Evolução do saldo</h2>
            <p>Saldo após as movimentações do período</p>
          </div>
          <span className="legend">Saldo disponível</span>
        </div>
        <div className="chart" role="img" aria-label={`Gráfico demonstrativo da evolução do saldo. Saldo inicial ${brl(s.opening)} e saldo disponível ${brl(s.available)}.`}>
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart
              data={points}
              margin={{ right: 12, left: 0, top: 12, bottom: 0 }}
            >
              <defs>
                <linearGradient id="wash" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0" stopColor="#61a98d" stopOpacity={0.35} />
                  <stop offset="1" stopColor="#61a98d" stopOpacity={0.02} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="4 5" vertical={false} />
              <XAxis dataKey="day" tickLine={false} axisLine={false} />
              <YAxis
                tickFormatter={(v) => Math.round(v / 100000) + " mil"}
                domain={["auto", "auto"]}
                tickLine={false}
                axisLine={false}
              />
              <Tooltip formatter={(v) => [brl(Number(v)), "Saldo"]} />
              <Area
                type="stepAfter"
                dataKey="saldo"
                stroke="#278263"
                strokeWidth={3}
                fill="url(#wash)"
                isAnimationActive={false}
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      </section>
      <section className="panel">
        <h2>Cobranças e pagamentos</h2>
        <dl className="forecast">
          <div>
            <dt>Cobranças recebidas</dt>
            <dd>{brl(s.summary.settled_charge_cents)}</dd>
          </div>
          <div>
            <dt>Cobranças pendentes</dt>
            <dd>{brl(s.summary.pending_charge_cents)}</dd>
          </div>
          <div>
            <dt>Pagamentos agendados</dt>
            <dd>{brl(s.summary.scheduled_payment_cents)}</dd>
          </div>
        </dl>
        <p className="syncnote">
          Conciliação pendente · {s.summary.unreconciled_count} lançamento
        </p>
      </section>
      <Ledger s={s} open={open} />
      <p className="syncnote">
        Base fictícia de {new Intl.DateTimeFormat("pt-BR", { dateStyle: "long", timeZone: "UTC" }).format(new Date(s.as_of + "T12:00:00Z"))} · Sem sincronização bancária
      </p>
    </>
  );
}
