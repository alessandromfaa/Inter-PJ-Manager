import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { zodResolver } from "@hookform/resolvers/zod";
import { QRCodeCanvas } from "qrcode.react";
import { z } from "zod";
import { api, ApiError, brl } from "../services/api";
import { Button } from "../components/button";
import type { Charge } from "./pix";

const formSchema = z.object({
  amount: z
    .string()
    .regex(/^\d{1,10}(,\d{1,2})?$/, "Informe um valor em reais, como 42,50.")
    .refine((v) => /[1-9]/.test(v), "O valor deve ser maior que zero."),
});
const intentSchema = z.object({
  intent_id: z.uuid(),
  original_cents: z.string().regex(/^[1-9]\d{0,11}$/),
});
type Intent = z.infer<typeof intentSchema>;

function loadIntent(key: string): Intent | null {
  const saved = sessionStorage.getItem(key);
  return saved ? intentSchema.parse(JSON.parse(saved)) : null;
}

export function MockPixCode({
  code,
}: {
  code: NonNullable<Charge["mock_code"]>;
}) {
  const [message, setMessage] = useState("");
  const expiry = Date.parse(code.expires_at);
  const [expired, setExpired] = useState(Date.now() >= expiry);
  useEffect(() => {
    const remaining = expiry - Date.now();
    const timer = window.setTimeout(
      () => setExpired(true),
      Math.max(remaining, 0),
    );
    return () => window.clearTimeout(timer);
  }, [expiry]);
  return (
    <div className="mock-code">
      <p>
        {expired ? "Código expirado em " : "Válido até "}
        {new Date(expiry).toLocaleString("pt-BR", {
          timeZone: "America/Fortaleza",
        })}{" "}
        (Fortaleza).
      </p>
      {!expired && (
        <>
          <p>QR Code fictício para demonstração. Não permite pagamento.</p>
          <QRCodeCanvas
            value={code.copy_paste}
            size={256}
            marginSize={4}
            level="M"
            role="img"
            aria-label="QR Code da cobrança fictícia"
          />
          <label>
            Pix Copia e Cola (MOCK)
            <textarea
              rows={3}
              readOnly
              value={code.copy_paste}
              onFocus={(e) => e.target.select()}
            />
          </label>
          <Button
            variant="outline"
            onClick={async () => {
              try {
                await navigator.clipboard.writeText(code.copy_paste);
                setMessage("Código copiado.");
              } catch {
                setMessage("Selecione e copie o código no campo acima.");
              }
            }}
          >
            Copiar código
          </Button>
          <span role="status">{message}</span>
        </>
      )}
    </div>
  );
}

export function MockPixIssue({ scope }: { scope: string }) {
  const storageKey = "inter-pj:mock:emissao:" + scope;
  const [draft, setDraft] = useState<Intent | null>(() => {
    try {
      return loadIntent(storageKey);
    } catch {
      return null;
    }
  });
  const cache = useQueryClient();
  const amount = draft?.original_cents.padStart(3, "0");
  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      amount: amount ? amount.slice(0, -2) + "," + amount.slice(-2) : "",
    },
  });
  const issue = useMutation({
    mutationFn: (fields: z.infer<typeof formSchema>) => {
      const [whole, fraction = ""] = fields.amount.split(",");
      const cents = BigInt(whole + fraction.padEnd(2, "0")).toString();
      let intent: Intent;
      try {
        intent = loadIntent(storageKey) ?? {
          intent_id: crypto.randomUUID(),
          original_cents: cents,
        };
        if (intent.original_cents !== cents)
          throw new Error("Valor divergente.");
        sessionStorage.setItem(storageKey, JSON.stringify(intent));
      } catch {
        throw new Error(
          "Não foi possível recuperar a intenção desta aba. Consulte o txid antes de iniciar uma nova emissão.",
        );
      }
      setDraft(intent);
      return api<Charge>("mock/pix/cobrancas", intent).catch(
        (error: unknown) => {
          if (error instanceof ApiError && error.status < 500) throw error;
          throw new Error(
            "A confirmação não chegou. Use Retomar emissão para verificar a mesma cobrança.",
          );
        },
      );
    },
    onSuccess: () => cache.invalidateQueries({ queryKey: ["auditoria"] }),
    onError: (err) => {
      if (err instanceof ApiError && err.status === 401)
        void cache.invalidateQueries({ queryKey: ["sessão"] });
    },
    retry: false,
    gcTime: 0,
  });
  const issued = issue.data;
  return (
    <section className="panel">
      <h2>
        {issued
          ? "Cobrança fictícia emitida"
          : "Emitir cobrança imediata em MOCK"}
      </h2>
      {issued ? (
        <>
          <p className="pix-id">
            Txid · <code>{issued.txid}</code>
          </p>
          <dl className="forecast">
            <div>
              <dt>Valor original</dt>
              <dd>{brl(issued.original_cents)}</dd>
            </div>
            <div>
              <dt>Situação</dt>
              <dd>Ativa</dd>
            </div>
          </dl>
          {issued.mock_code && (
            <MockPixCode key={issued.txid} code={issued.mock_code} />
          )}
          <Button
            variant="outline"
            onClick={() => {
              sessionStorage.removeItem(storageKey);
              setDraft(null);
              issue.reset();
              form.reset({ amount: "" });
            }}
          >
            Iniciar outra emissão
          </Button>
        </>
      ) : (
        <>
          <p>
            A cobrança fica disponível para consulta na empresa de demonstração
            por txid.
          </p>
          {draft && (
            <p className="pix-id">
              Intenção salva nesta aba ·{" "}
              <code>{"M" + draft.intent_id.replaceAll("-", "")}</code>
            </p>
          )}
          <form
            noValidate
            onSubmit={form.handleSubmit((fields) => issue.mutate(fields))}
          >
            <label>
              Valor da cobrança (R$)
              <input
                inputMode="decimal"
                autoComplete="off"
                placeholder="0,00"
                readOnly={!!draft || issue.isPending}
                {...form.register("amount")}
              />
            </label>
            {form.formState.errors.amount && (
              <small role="alert">{form.formState.errors.amount.message}</small>
            )}
            <Button disabled={issue.isPending}>
              {issue.isPending
                ? "Processando…"
                : draft
                  ? "Retomar emissão"
                  : "Emitir cobrança fictícia"}
            </Button>
          </form>
          {issue.error && <p role="alert">{issue.error.message}</p>}
        </>
      )}
    </section>
  );
}
