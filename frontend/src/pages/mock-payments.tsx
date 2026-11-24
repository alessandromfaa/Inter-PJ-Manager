import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { Button } from "../components/button";
import {
  api,
  ApiError,
  brl,
  environmentLabel,
  type Identity,
} from "../services/api";

const route = "mock/pagamentos/pix";
const maxPage = 1000;
const dateTime = new Intl.DateTimeFormat("pt-BR", {
  dateStyle: "short",
  timeStyle: "short",
  timeZone: "America/Fortaleza",
});
const statusLabels = {
  RASCUNHO: "Rascunho",
  EM_REVISAO: "Em revisão",
};
const draftSchema = z.object({
  intent_id: z.uuid(),
  destinatario: z.enum(["FORNECEDOR_DEMO", "SERVICOS_DEMO"]),
  valor_centavos: z.string().regex(/^[1-9]\d{0,11}$/),
});
type Draft = z.infer<typeof draftSchema>;
type Recipient = { id: Draft["destinatario"]; nome: string };
type Intent = {
  id: string;
  destinatario: Draft["destinatario"];
  valor_centavos: string;
  situacao: keyof typeof statusLabels;
  criado_em: string;
  pode_revisar: boolean;
};
const intentSchema: z.ZodType<Intent> = z.object({
  id: z.uuid(),
  destinatario: z.enum(["FORNECEDOR_DEMO", "SERVICOS_DEMO"]),
  valor_centavos: z.string().regex(/^[1-9]\d{0,11}$/),
  situacao: z.enum(["RASCUNHO", "EM_REVISAO"]),
  criado_em: z.string().datetime(),
  pode_revisar: z.boolean(),
});
type Intents = {
  destinatarios: Recipient[];
  intencoes: Intent[];
  pode_preparar: boolean;
  tem_proxima: boolean;
};
const formSchema = z.object({
  recipient: z.string().min(1, "Selecione um destinatário fictício."),
  amount: z
    .string()
    .trim()
    .regex(/^\d{1,10}(,\d{1,2})?$/, "Informe um valor em reais, como 42,50.")
    .refine((v) => /[1-9]/.test(v), "O valor deve ser maior que zero."),
});
type Form = z.infer<typeof formSchema>;

function readDraft(key: string): Draft | null {
  const saved = sessionStorage.getItem(key);
  return saved ? draftSchema.parse(JSON.parse(saved)) : null;
}

function apiMessage(error: Error, fallback: string) {
  return error instanceof ApiError && error.status < 500
    ? error.message
    : fallback;
}

function PrepareIntent({
  storageKey,
  recipients,
  onSaved,
  onExpired,
}: {
  storageKey: string;
  recipients: Recipient[];
  onSaved: () => void;
  onExpired: () => void;
}) {
  const [initial] = useState(() => {
    try {
      return { draft: readDraft(storageKey), storageError: false };
    } catch {
      return { draft: null, storageError: true };
    }
  });
  const [draft, setDraft] = useState(initial.draft);
  const [savedInTab, setSavedInTab] = useState(false);
  const form = useForm<Form>({
    resolver: zodResolver(formSchema),
    defaultValues: { recipient: "", amount: "" },
  });
  const create = useMutation({
    mutationFn: async (fields: Form | null) => {
      let intent = readDraft(storageKey);
      if (!intent && fields) {
        const [whole, fraction = ""] = fields.amount.split(",");
        intent = draftSchema.parse({
          intent_id: crypto.randomUUID(),
          destinatario: fields.recipient,
          valor_centavos: BigInt(whole + fraction.padEnd(2, "0")).toString(),
        });
        try {
          sessionStorage.setItem(storageKey, JSON.stringify(intent));
        } catch {
          throw new Error(
            "Não foi possível guardar a solicitação nesta aba. Nenhum rascunho foi enviado.",
          );
        }
        setDraft(intent);
      }
      if (!intent) throw new Error("Rascunho indisponível nesta aba.");
      return api<unknown>(route, intent).then((response) => {
        const parsed = intentSchema.parse(response);
        if (
          parsed.destinatario !== intent.destinatario ||
          parsed.valor_centavos !== intent.valor_centavos
        )
          throw new Error("A confirmação não corresponde ao rascunho enviado.");
        return parsed;
      }).catch((error: unknown) => {
        if (error instanceof ApiError && error.status < 500) throw error;
        throw new Error(
          "A confirmação não chegou. Use Retomar salvamento para verificar o mesmo rascunho.",
        );
      });
    },
    onSuccess: () => {
      form.reset();
      try {
        sessionStorage.removeItem(storageKey);
        setDraft(null);
      } catch {
        setSavedInTab(true);
      }
      onSaved();
    },
    onError: (error) => {
      if (error instanceof ApiError && error.status === 401) onExpired();
    },
    retry: false,
    gcTime: 0,
  });
  const errors = form.formState.errors;

  return (
    <section className="panel" aria-labelledby="preparar-pix">
      <h2 id="preparar-pix">Preparar intenção Pix</h2>
      {initial.storageError ? (
        <p role="alert">
          Não foi possível recuperar a solicitação desta aba. Consulte a lista
          de intenções antes de preparar outro rascunho.
        </p>
      ) : draft ? (
        <>
          <dl className="forecast">
            <div>
              <dt>Destinatário fictício</dt>
              <dd>
                {recipients.find((r) => r.id === draft.destinatario)?.nome ??
                  draft.destinatario}
              </dd>
            </div>
            <div>
              <dt>Valor</dt>
              <dd>{brl(draft.valor_centavos)}</dd>
            </div>
          </dl>
          {savedInTab ? (
            <p role="alert">
              Rascunho salvo. A aba não conseguiu limpar a solicitação local.
            </p>
          ) : (
            <Button
              type="button"
              disabled={create.isPending}
              onClick={() => create.mutate(null)}
            >
              {create.isPending ? "Salvando…" : "Retomar salvamento"}
            </Button>
          )}
        </>
      ) : (
        <form
          className="mock-payment-form"
          noValidate
          onSubmit={form.handleSubmit((fields) => create.mutate(fields))}
        >
          <label>
            Destinatário fictício
            <select
              {...form.register("recipient")}
              aria-invalid={!!errors.recipient}
              aria-describedby={
                errors.recipient ? "pix-destinatario-erro" : undefined
              }
            >
              <option value="">Selecione um destinatário</option>
              {recipients.map((recipient) => (
                <option key={recipient.id} value={recipient.id}>
                  {recipient.nome}
                </option>
              ))}
            </select>
          </label>
          {errors.recipient && (
            <small id="pix-destinatario-erro" role="alert">
              {errors.recipient.message}
            </small>
          )}
          <label>
            Valor em reais
            <input
              {...form.register("amount")}
              inputMode="decimal"
              autoComplete="off"
              placeholder="0,00"
              aria-invalid={!!errors.amount}
              aria-describedby={errors.amount ? "pix-valor-erro" : undefined}
            />
          </label>
          {errors.amount && (
            <small id="pix-valor-erro" role="alert">
              {errors.amount.message}
            </small>
          )}
          <Button type="submit" disabled={create.isPending}>
            {create.isPending ? "Salvando…" : "Salvar rascunho"}
          </Button>
        </form>
      )}
      {create.error && <p role="alert">{create.error.message}</p>}
    </section>
  );
}

export function MockPayments({
  user,
  onExpired,
}: {
  user: Identity;
  onExpired: () => void;
}) {
  const cache = useQueryClient();
  const [page, setPage] = useState(1);
  const [confirming, setConfirming] = useState<string | null>(null);
  const [message, setMessage] = useState("");
  const key = ["intenções-pix-mock", user.company_id, user.id];
  const rows = useQuery({
    queryKey: [...key, page],
    queryFn: () => api<Intents>(route + "?page=" + page),
  });
  useEffect(() => {
    if (rows.error instanceof ApiError && rows.error.status === 401) onExpired();
  }, [rows.error, onExpired]);
  const refresh = () => {
    void cache.invalidateQueries({ queryKey: key });
    void cache.invalidateQueries({ queryKey: ["auditoria"] });
  };
  const review = useMutation({
    mutationFn: async ({ id, destinatario, valor_centavos }: Intent) => {
      const response = await api<unknown>(route + "/revisar", {
        id,
        destinatario,
        valor_centavos,
      });
      const parsed = intentSchema.parse(response);
      if (
        parsed.id !== id ||
        parsed.destinatario !== destinatario ||
        parsed.valor_centavos !== valor_centavos ||
        parsed.situacao !== "EM_REVISAO"
      )
        throw new Error("A confirmação de revisão não corresponde à intenção.");
      return parsed;
    },
    onSuccess: () => {
      setConfirming(null);
      setMessage("Intenção encaminhada para revisão.");
      refresh();
    },
    onError: (error) => {
      if (error instanceof ApiError && error.status === 401) onExpired();
    },
    retry: false,
    gcTime: 0,
  });
  const changePage = (nextPage: number) => {
    setPage(nextPage);
    setConfirming(null);
    review.reset();
  };

  return (
    <>
      <section className="panel" aria-label="Ambiente dos pagamentos Pix">
        <span className="badge">{environmentLabel("MOCK")}</span>
        <p>
          Destinatários e valores são fictícios. Salvar e encaminhar para
          revisão não envia dinheiro.
        </p>
      </section>
      {rows.data?.pode_preparar && !rows.isError && (
        <PrepareIntent
          storageKey={
            "inter-pj:mock:intencao-pix:" + user.company_id + ":" + user.id
          }
          recipients={rows.data.destinatarios}
          onExpired={onExpired}
          onSaved={() => {
            setMessage("Rascunho salvo.");
            setPage(1);
            refresh();
          }}
        />
      )}
      {message && <p role="status">{message}</p>}
      <section className="panel" aria-labelledby="intencoes-pix">
        <div className="panelhead">
          <h2 id="intencoes-pix">Intenções de pagamento</h2>
          <Button
            type="button"
            variant="outline"
            disabled={rows.isFetching || review.isPending}
            onClick={() => void rows.refetch()}
          >
            Atualizar
          </Button>
        </div>
        {rows.isPending ? (
          <p role="status">Carregando intenções…</p>
        ) : rows.isError ? (
          <p role="alert">
            {apiMessage(rows.error, "Não foi possível consultar as intenções.")}
          </p>
        ) : (
          <>
            {rows.data.intencoes.length === 0 ? (
              <p>Nenhuma intenção nesta página.</p>
            ) : (
              <ul className="mock-payment-list">
                {rows.data.intencoes.map((intent) => {
                  const recipient = rows.data.destinatarios.find(
                    (r) => r.id === intent.destinatario,
                  );
                  const expanded = confirming === intent.id;
                  const controls = "revisar-pix-" + intent.id;
                  return (
                    <li className="mock-payment-item" key={intent.id}>
                      <dl className="forecast">
                        <div>
                          <dt>Destinatário fictício</dt>
                          <dd>{recipient?.nome ?? intent.destinatario}</dd>
                        </div>
                        <div>
                          <dt>Valor</dt>
                          <dd>{brl(intent.valor_centavos)}</dd>
                        </div>
                        <div>
                          <dt>Situação</dt>
                          <dd>{statusLabels[intent.situacao]}</dd>
                        </div>
                        <div>
                          <dt>Criação (Fortaleza)</dt>
                          <dd>
                            <time dateTime={intent.criado_em}>
                              {dateTime.format(new Date(intent.criado_em))}
                            </time>
                          </dd>
                        </div>
                      </dl>
                      {intent.pode_revisar && (
                        <>
                          <Button
                            type="button"
                            variant="outline"
                            aria-expanded={expanded}
                            aria-controls={controls}
                            disabled={review.isPending}
                            onClick={() => {
                              review.reset();
                              setMessage("");
                              setConfirming(expanded ? null : intent.id);
                            }}
                          >
                            {expanded ? "Cancelar revisão" : "Revisar rascunho"}
                          </Button>
                          <div id={controls} hidden={!expanded}>
                            {expanded && (
                              <div className="mock-payment-confirmation">
                                <p>
                                  Encaminhar {brl(intent.valor_centavos)} para
                                  revisão da intenção de{" "}
                                  <strong>
                                    {recipient?.nome ?? intent.destinatario}
                                  </strong>
                                  ?
                                </p>
                                <Button
                                  type="button"
                                  disabled={review.isPending}
                                  onClick={() => review.mutate(intent)}
                                >
                                  {review.isPending
                                    ? "Registrando…"
                                    : "Confirmar revisão"}
                                </Button>
                                {review.error && (
                                  <p role="alert">
                                    {apiMessage(
                                      review.error,
                                      "A confirmação não chegou. Atualize a lista ou tente confirmar novamente.",
                                    )}
                                  </p>
                                )}
                              </div>
                            )}
                          </div>
                        </>
                      )}
                    </li>
                  );
                })}
              </ul>
            )}
            {(page > 1 || rows.data.tem_proxima) && (
              <div
                className="filters"
                role="group"
                aria-label="Páginas das intenções Pix"
              >
                <Button
                  type="button"
                  variant="outline"
                  disabled={page === 1 || review.isPending}
                  onClick={() => changePage(page - 1)}
                >
                  Anterior
                </Button>
                <span>Página {page}</span>
                <Button
                  type="button"
                  variant="outline"
                  disabled={
                    !rows.data.tem_proxima || page >= maxPage || review.isPending
                  }
                  onClick={() => changePage(page + 1)}
                >
                  Próxima
                </Button>
              </div>
            )}
          </>
        )}
      </section>
    </>
  );
}
