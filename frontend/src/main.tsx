import React, { useState, lazy, Suspense } from "react";
import { createRoot } from "react-dom/client";
import {
  QueryClient,
  QueryClientProvider,
  useQuery,
  useQueryClient,
  useMutation,
} from "@tanstack/react-query";
import {
  LayoutDashboard,
  ListFilter,
  ShieldCheck,
  LogOut,
  SunMoon,
  CalendarDays,
} from "lucide-react";
import { Login } from "./pages/login";
const Overview = lazy(() =>
  import("./pages/overview").then((m) => ({ default: m.Overview })),
);
const Ledger = lazy(() =>
  import("./pages/overview").then((m) => ({ default: m.Ledger })),
);
import { api, ApiError, type Identity, type Snapshot } from "./services/api";
import { Brand } from "./components/brand";
import "./style.css";
const client = new QueryClient({
  defaultOptions: { queries: { retry: false, refetchOnWindowFocus: true } },
});
function Failure({ retry }: { retry: () => void }) {
  return (
    <p role="alert">
      Serviço indisponível.{" "}
      <button className="link" onClick={retry}>
        Tentar novamente
      </button>
    </p>
  );
}
function App() {
  const cache = useQueryClient();
  const session = useQuery({
    queryKey: ["sessão"],
    queryFn: () => api<Identity>("session"),
  });
  const [view, setView] = useState("Visão geral");
  const [dark, setDark] = useState(localStorage.getItem("tema") === "escuro");
  const reset = () => {
    setView("Visão geral");
    cache.clear();
    void session.refetch();
  };
  const logout = useMutation({
    mutationFn: () => api("logout", {}),
    onSuccess: reset,
  });
  const canAudit = session.data?.roles.some(
    (r) => r === "ADMIN" || r === "AUDITOR",
  );
  const data = useQuery({
    queryKey: ["painel", session.data?.email],
    queryFn: () => api<Snapshot>("dashboard"),
    enabled: !!session.data,
  });
  const audit = useQuery({
    queryKey: ["auditoria", session.data?.email],
    queryFn: () => api<{ at: string; action: string }[]>("auditoria"),
    enabled: !!canAudit && view === "Auditoria",
  });
  if (session.isPending)
    return <main className="status">Carregando sessão…</main>;
  if (
    [session.error, data.error, audit.error].some(
      (e) => e instanceof ApiError && e.status === 401,
    )
  )
    return <Login done={reset} />;
  if (!session.data) return <Failure retry={() => void session.refetch()} />;
  return (
    <div className={"shell " + (dark ? "dark" : "")}>
      <aside>
        <Brand />
        <div className="navgroup">ÁREA FINANCEIRA</div>
        <nav>
          {[
            { name: "Visão geral", icon: LayoutDashboard },
            { name: "Extrato", icon: ListFilter },
            ...(canAudit ? [{ name: "Auditoria", icon: ShieldCheck }] : []),
          ].map(({ name, icon: Icon }) => (
            <button
              aria-current={view === name ? "page" : undefined}
              key={name}
              onClick={() => setView(name)}
            >
              <Icon size={18} />
              {name}
            </button>
          ))}
        </nav>
        <div className="asidefoot">
          <div className="badge">MOCK · Demonstração</div>
          <p>Dados bancários fictícios</p>
        </div>
      </aside>
      <div className="workspace">
        <header>
          <span>{session.data.company}</span>
          <div>
            <button
              title="Alternar tema"
              aria-label="Alternar tema"
              onClick={() => {
                localStorage.setItem("tema", dark ? "claro" : "escuro");
                setDark(!dark);
              }}
            >
              <SunMoon size={19} />
            </button>
            <button onClick={() => logout.mutate()} disabled={logout.isPending}>
              <LogOut size={16} /> Sair
            </button>
          </div>
        </header>
        <main className="content">
          {logout.error && (
            <p role="alert">Falha ao encerrar a sessão. Tente novamente.</p>
          )}
          <div className="pagehead">
            <div>
              <span className="eyebrow">PAINEL FINANCEIRO</span>
              <h1>{view}</h1>
            </div>
            {view !== "Auditoria" && (
              <span className="period">
                <CalendarDays size={16} /> Novembro de 2026
              </span>
            )}
          </div>
          {view === "Auditoria" ? (
            <section className="panel">
              <h2>Atividade da empresa</h2>
              {audit.isPending ? (
                "Carregando…"
              ) : audit.error ? (
                <Failure retry={() => void audit.refetch()} />
              ) : (
                audit.data.map((a, i) => (
                  <p className="event" key={i}>
                    <strong>{a.action}</strong>
                    {new Date(a.at).toLocaleString("pt-BR", {
                      timeZone: "America/Fortaleza",
                    })}
                  </p>
                ))
              )}
            </section>
          ) : data.isPending ? (
            "Carregando painel…"
          ) : data.error ? (
            <Failure retry={() => void data.refetch()} />
          ) : view === "Extrato" ? (
            <Ledger s={data.data} full />
          ) : (
            <Overview s={data.data} open={() => setView("Extrato")} />
          )}
        </main>
      </div>
    </div>
  );
}
createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <QueryClientProvider client={client}>
      <Suspense fallback={<main className="status">Carregando painel…</main>}>
        <App />
      </Suspense>
    </QueryClientProvider>
  </React.StrictMode>,
);
