import { useState } from "react";
import { MockPixCode, MockPixIssue } from "./mock-pix";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { Button } from "../components/button";
import {
  api,
  ApiError,
  brl,
  environmentLabel,
  type Mode,
} from "../services/api";

const schema = z.object({
  txid: z
    .string()
    .regex(/^[a-zA-Z0-9]{26,35}$/, "Informe de 26 a 35 letras ou números."),
});
const situations = {
  ATIVA: "Ativa",
  CONCLUIDA: "Concluída",
  REMOVIDA_PELO_USUARIO_RECEBEDOR: "Removida pelo recebedor",
  REMOVIDA_PELO_PSP: "Removida pelo prestador de serviços de pagamento",
};
export type Charge = {
  mock_code?: { copy_paste: string; expires_at: string };
  txid: string;
  source: Mode;
  original_cents: string;
  status: keyof typeof situations;
  revision: number;
};

export function Pix({
  mode,
  canIssue,
  scope,
}: {
  mode: Mode;
  canIssue: boolean;
  scope: string;
}) {
  const [issuing, setIssuing] = useState(false);
  const allowIssue = canIssue && mode === "MOCK";
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<z.infer<typeof schema>>({
    resolver: zodResolver(schema),
    defaultValues: {
      txid: mode === "MOCK" ? "DEMO000000000000000000000001" : "",
    },
  });
  const cache = useQueryClient();
  const lookup = useMutation({
    mutationFn: (fields: z.infer<typeof schema>) =>
      api<Charge>("pix/cobrancas/consultar", fields),
    onSuccess: () => cache.invalidateQueries({ queryKey: ["auditoria"] }),
    onError: (err) => {
      if (err instanceof ApiError && err.status === 401)
        void cache.invalidateQueries({ queryKey: ["sessão"] });
    },
    retry: false,
    gcTime: 0,
  });
  const charge = lookup.data;
  return (
    <>
      {allowIssue && (
        <Button
          variant="outline"
          onClick={() => {
            setIssuing(!issuing);
            lookup.reset();
          }}
        >
          {issuing ? "Voltar à consulta" : "Nova cobrança em MOCK"}
        </Button>
      )}
      {allowIssue && issuing ? (
        <MockPixIssue scope={scope} />
      ) : (
        <>
          <section className="panel">
            <h2>Consultar cobrança imediata</h2>
            <form
              noValidate
              onSubmit={handleSubmit((fields) => lookup.mutate(fields))}
            >
              <label>
                Identificador da cobrança (txid)
                <input
                  autoComplete="off"
                  spellCheck={false}
                  maxLength={35}
                  readOnly={lookup.isPending}
                  {...register("txid", { onChange: () => lookup.reset() })}
                />
              </label>
              {errors.txid && <small role="alert">{errors.txid.message}</small>}
              <Button disabled={lookup.isPending}>
                {lookup.isPending ? "Consultando…" : "Consultar cobrança"}
              </Button>
            </form>
            {lookup.error && <p role="alert">{lookup.error.message}</p>}
          </section>
          {charge && (
            <section className="panel" aria-live="polite">
              <div className="panelhead">
                <h2>
                  Cobrança{" "}
                  {situations[charge.status].toLocaleLowerCase("pt-BR")}
                </h2>
                <span className="badge">{environmentLabel(charge.source)}</span>
              </div>
              <dl className="forecast">
                <div>
                  <dt>Valor original</dt>
                  <dd>{brl(charge.original_cents)}</dd>
                </div>
                <div>
                  <dt>Revisão</dt>
                  <dd>{charge.revision}</dd>
                </div>
              </dl>
              <p className="pix-id">Txid · {charge.txid}</p>
              {charge.mock_code && (
                <MockPixCode key={charge.txid} code={charge.mock_code} />
              )}
            </section>
          )}
        </>
      )}
    </>
  );
}
