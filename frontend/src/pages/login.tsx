import { environmentLabel, type Mode } from "../services/api";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation } from "@tanstack/react-query";
import { ShieldCheck, LockKeyhole } from "lucide-react";
import { api } from "../services/api";
import { Button } from "../components/button";
import { Brand } from "../components/brand";
const schema = z.object({
  email: z.email("Informe um e-mail válido."),
  password: z.string().min(1, "Informe sua senha."),
});
export function Login({
  done,
  mode,
  expired = false,
}: {
  done: () => void;
  mode: Mode;
  expired?: boolean;
}) {
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<z.infer<typeof schema>>({ resolver: zodResolver(schema) });
  const login = useMutation({
    mutationFn: (data: z.infer<typeof schema>) => api("login", data),
    onSuccess: done,
  });
  return (
    <main className="entrance">
      <section className="intro">
        <Brand />
        <div>
          <span className="eyebrow">GESTÃO FINANCEIRA EMPRESARIAL</span>
          <h1>
            Finanças da
            <br />
            sua empresa.
          </h1>
          <p>
            Acompanhe saldos, movimentações e fluxo de caixa em um só painel.
          </p>
          <svg className="motif" viewBox="0 0 440 100" aria-hidden="true">
            {[24, 42, 34, 58, 46, 65, 61, 80, 68, 90, 84, 98].map((h, i) => (
              <rect
                key={i}
                x={i * 37}
                y={100 - h}
                width="25"
                height={h}
                rx="4"
                fill={i > 8 ? "#e7ac64" : "#477b6d"}
              />
            ))}
          </svg>
        </div>
        <small>Plataforma independente · Inter PJ Manager</small>
      </section>
      <section className="login">
        <span className="badge">{environmentLabel(mode)}</span>
        <h2>Acessar painel</h2>
        {expired ? (
          <p role="alert">Sua sessão expirou. Entre novamente para continuar.</p>
        ) : (
          <p>Entre com suas credenciais da empresa.</p>
        )}
        <form onSubmit={handleSubmit((data) => login.mutate(data))} noValidate>
          <label>
            E-mail
            <input
              type="email"
              autoComplete="username"
              aria-invalid={!!errors.email}
              aria-describedby={errors.email ? "login-email-erro" : undefined}
              {...register("email")}
            />
          </label>
          {errors.email && <small id="login-email-erro" role="alert">{errors.email.message}</small>}
          <label>
            Senha
            <input
              type="password"
              autoComplete="current-password"
              aria-invalid={!!errors.password}
              aria-describedby={errors.password ? "login-senha-erro" : undefined}
              {...register("password")}
            />
          </label>
          {errors.password && (
            <small id="login-senha-erro" role="alert">{errors.password.message}</small>
          )}
          {login.error && <p role="alert">{login.error.message}</p>}
          <Button disabled={login.isPending}>
            <LockKeyhole size={16} />
            {login.isPending ? "Entrando…" : "Entrar"}
          </Button>
        </form>
        <footer>
          <ShieldCheck size={17} />{" "}
          {mode === "PRODUCTION"
            ? "Acesso empresarial de leitura"
            : mode === "SANDBOX" ? "Ambiente de homologação bancária" : "Ambiente com dados demonstrativos fictícios"}
        </footer>
      </section>
    </main>
  );
}
