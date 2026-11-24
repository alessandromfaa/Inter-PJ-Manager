import { useState } from "react";
import { useForm } from "react-hook-form";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { Button } from "../components/button";
import { api, brl, type Mode, type Identity } from "../services/api";

type List = "clientes" | "pix" | "cobrancas" | "eventos" | "confirmacoes";
type Item = {
  id: string;
  name?: string;
  status?: string;
  cents?: string;
  due_on?: string;
  channel?: string;
  family?: string;
  reference?: string;
  txid?: string;
  reconciliation?: string;
  attempts?: number;
  error?: string;
};
const tabs: Record<List, string> = {
  clientes: "Clientes",
  pix: "Cobranças Pix emitidas",
  cobrancas: "Cobranças V3",
  eventos: "Notificações",
  confirmacoes: "Conciliações",
};
const billSchema = z.object({
  amount: z
    .string()
    .regex(/^\d{1,8}(,\d{1,2})?$/, "Informe um valor em reais.")
    .refine((x) => {
      const [a, b = ""] = x.split(",");
      return /^\d+$/.test(a + b) && BigInt(a + b.padEnd(2, "0")) >= 250n;
    }, "Valor mínimo de R$ 2,50."),
  due: z.iso.date("Informe o vencimento."),
});
function BillingForm({
  customer,
  done,
}: {
  customer: Item;
  done: (code: string) => void;
}) {
  const [intent] = useState(() => crypto.randomUUID());
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<z.infer<typeof billSchema>>({
    resolver: zodResolver(billSchema),
  });
  const send = useMutation({
    mutationFn: ({ amount, due }: z.infer<typeof billSchema>) => {
      const [w, f = ""] = amount.split(",");
      return api<{ codigo_solicitacao: string }>("mock/cobrancas-v3", {
        intent_id: intent,
        customer_id: customer.id,
        original_cents: BigInt(w + f.padEnd(2, "0")).toString(),
        due_on: due,
      });
    },
    onSuccess: (r) => done(r.codigo_solicitacao),
    retry: false,
  });
  return (
    <form onSubmit={handleSubmit((f) => send.mutate(f))}>
      <h2>Cobrança V3 para {customer.name}</h2>
      <label>
        Valor nominal (R$)
        <input inputMode="decimal" {...register("amount")} />
      </label>
      {errors.amount && <small role="alert">{errors.amount.message}</small>}
      <label>
        Vencimento
        <input type="date" {...register("due")} />
      </label>
      {errors.due && <small role="alert">{errors.due.message}</small>}
      <Button disabled={send.isPending}>Solicitar emissão em MOCK</Button>
      {send.error && <p role="alert">{send.error.message}</p>}
    </form>
  );
}

export function Recebimentos({ mode, user }: { mode: Mode; user: Identity }) {
  const [list, setList] = useState<List>("clientes"),
    [page, setPage] = useState(0);
  const [selected, setSelected] = useState<Item | null>(null),
    [message, setMessage] = useState("");
  const [name, setName] = useState(""),
    [clientId, setClientId] = useState(() => crypto.randomUUID());
  const write = user.roles.some((r) => r === "ADMIN" || r === "OPERATOR");
  const cache = useQueryClient();
  const key = ["recebimentos", user.company_id, user.id, mode];
  const rows = useQuery({
    queryKey: [...key, list, page],
    queryFn: () =>
      api<{ items: Item[]; has_more: boolean }>(
        `recebimentos?lista=${list}&pagina=${page}`,
      ),
    refetchInterval: list === "clientes" ? false : 4000,
  });
  const op = useMutation({
    mutationFn: ({ path, body }: { path: string; body: unknown }) =>
      api<unknown>(path, body),
    retry: false,
    onSuccess: (_, input) => {
      if (input.path === "clientes") {
        setName("");
        setClientId(crypto.randomUUID());
      }
      setMessage(
        input.path === "clientes"
          ? "Cliente cadastrado."
          : "Solicitação registrada. Acompanhe o processamento em Notificações.",
      );
      void cache.invalidateQueries({ queryKey: key });
      void cache.invalidateQueries({ queryKey: ["auditoria"] });
    },
  });
  const change = (v: List) => {
    setList(v);
    setPage(0);
    setSelected(null);
    setMessage("");
    op.reset();
  };
  const act = (path: string, body: unknown) => {
    setMessage("");
    op.mutate({ path, body });
  };
  return (
    <>
      <div className="filters" aria-label="Seções de recebimentos">
        {(Object.keys(tabs) as List[])
          .filter((t) => mode === "MOCK" || !["pix", "cobrancas"].includes(t))
          .map((t) => (
            <Button
              key={t}
              variant={list === t ? "default" : "outline"}
              aria-pressed={list === t}
              onClick={() => change(t)}
            >
              {tabs[t]}
            </Button>
          ))}
      </div>
      {list === "clientes" && write && (
        <section className="panel">
          <form
            onSubmit={(e) => {
              e.preventDefault();
              act("clientes", { id: clientId, name: name.trim() });
            }}
          >
            <label>
              Nome do cliente
              <input
                value={name}
                maxLength={120}
                required
                onChange={(e) => setName(e.target.value)}
              />
            </label>
            <Button disabled={op.isPending || !name.trim()}>
              Cadastrar cliente
            </Button>
          </form>
        </section>
      )}
      {selected && (
        <section className="panel">
          <BillingForm
            key={selected.id}
            customer={selected}
            done={(code) => {
              change("cobrancas");
              setMessage("Solicitação aceita · " + code);
              void cache.invalidateQueries({ queryKey: key });
            }}
          />
        </section>
      )}
      {message && (
        <p role="status" className="pix-id">
          {message}
        </p>
      )}
      {op.error && <p role="alert">{op.error.message}</p>}
      <section className="panel">
        <div className="panelhead">
          <h2>{tabs[list]}</h2>
          <Button variant="outline" onClick={() => void rows.refetch()}>
            Atualizar
          </Button>
        </div>
        {rows.isPending ? (
          <p>Carregando…</p>
        ) : rows.error ? (
          <p role="alert">{rows.error.message}</p>
        ) : (
          <>
            {rows.data.items.length === 0 ? (
              <p>Nenhum registro nesta página.</p>
            ) : (
              <div className="scroll">
                <table>
                  <thead>
                    <tr>
                      <th>{list === "clientes" ? "Nome" : "Referência"}</th>
                      <th>Detalhes</th>
                      <th>Ações</th>
                    </tr>
                  </thead>
                  <tbody>
                    {rows.data.items.map((row) => (
                      <tr key={(row.family ?? "") + row.id}>
                        <td>
                          {list === "clientes"
                            ? row.name
                            : (row.reference ?? row.id)}
                        </td>
                        <td>
                          {[
                            row.family,
                            row.status?.replaceAll("_", " "),
                            row.reconciliation,
                            row.txid,
                            row.name && list !== "clientes" ? row.name : null,
                            row.due_on?.split("-").reverse().join("/"),
                            row.cents !== undefined ? brl(row.cents) : null,
                            row.channel,
                            row.attempts !== undefined
                              ? `Tentativas: ${row.attempts}`
                              : null,
                            row.error,
                          ]
                            .filter(Boolean)
                            .join(" · ") || "Cadastro local"}
                        </td>
                        <td>
                          {write && mode === "MOCK" && list === "clientes" && (
                            <Button
                              variant="outline"
                              onClick={() => {
                                setSelected(row);
                                setMessage("");
                              }}
                            >
                              Preparar V3
                            </Button>
                          )}
                          {write &&
                            list === "pix" &&
                            row.status === "ATIVA" && (
                              <Button
                                disabled={op.isPending}
                                onClick={() =>
                                  act("mock/recebimentos/pix", { txid: row.id })
                                }
                              >
                                Simular recebimento Pix
                              </Button>
                            )}
                          {write &&
                            list === "cobrancas" &&
                            row.status === "A_RECEBER" &&
                            (["BOLETO", "PIX", "CANCELAR"] as const).map(
                              (action) => (
                                <Button
                                  variant="outline"
                                  key={action}
                                  disabled={op.isPending}
                                  onClick={() =>
                                    act("mock/cobrancas-v3/atualizar", {
                                      code: row.id,
                                      action,
                                    })
                                  }
                                >
                                  {action === "CANCELAR"
                                    ? "Cancelar em MOCK"
                                    : "Simular recebimento por " + action}
                                </Button>
                              ),
                            )}
                          {list === "eventos" &&
                            row.status === "ESGOTADO" &&
                            user.roles.includes("ADMIN") && (
                              <Button
                                onClick={() =>
                                  act("recebimentos/reprocessar", {
                                    id: row.id,
                                  })
                                }
                              >
                                Reprocessar
                              </Button>
                            )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
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
                disabled={!rows.data.has_more}
                onClick={() => setPage(page + 1)}
              >
                Próxima
              </Button>
            </div>
          </>
        )}
      </section>
    </>
  );
}
