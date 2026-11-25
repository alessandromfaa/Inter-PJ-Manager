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
  ScanLine,
  Receipt,
  ShieldCheck,
  LogOut,
  SunMoon,
  CalendarDays,
  Wallet,
  Repeat,
  Menu,
  X,
} from "lucide-react";
import { Recebimentos } from "./pages/recebimentos";
import { MockPayments } from "./pages/mock-payments";
import { Financeiro } from "./pages/financeiro";
import { PixAutomatico } from "./pages/pix-automatico";
import { Pix } from "./pages/pix";
import { Banking } from "./pages/banking";
import { Login } from "./pages/login";
const Overview = lazy(() =>
  import("./pages/overview").then((m) => ({ default: m.Overview })),
);
const Ledger = lazy(() =>
  import("./pages/overview").then((m) => ({ default: m.Ledger })),
);
import {
  api,
  ApiError,
  type Identity,
  type Snapshot,
  type Mode,
  type LiveSnapshot,
  environmentLabel,
} from "./services/api";
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
  const mode = useQuery({
    queryKey: ["ambiente"],
    queryFn: () => api<Mode>("ambiente"),
  });
  const session = useQuery({
    queryKey: ["sessão"],
    queryFn: () => api<Identity>("session"),
  });
  const [view, setView] = useState("Visão geral");
  const [dark, setDark] = useState(localStorage.getItem("tema") === "escuro");
  const [menuOpen, setMenuOpen] = useState(false);
  const [sessionExpired, setSessionExpired] = useState(false);
  const reset = () => {
    setSessionExpired(false);
    setView("Visão geral");
    cache.clear();
    void session.refetch();
  };
  const logout = useMutation({
    mutationFn: () => api<void>("logout", {}, { allowEmpty: true }),
    onSuccess: reset,
  });
  const canAudit = session.data?.roles.some(
    (r) => r === "ADMIN" || r === "AUDITOR",
  );
  const data = useQuery({
    queryKey: ["painel", session.data?.email],
    queryFn: () => api<Snapshot | LiveSnapshot>("dashboard"),
    enabled: !!session.data,
  });
  const audit = useQuery({
    queryKey: ["auditoria", session.data?.email],
    queryFn: () => api<{ at: string; action: string }[]>("auditoria"),
    enabled: !!canAudit && view === "Auditoria",
  });
  if (mode.isError) return <Failure retry={() => void mode.refetch()} />;
  if (mode.isPending || session.isPending)
    return <main className="status">Carregando sessão…</main>;
  if (
    sessionExpired ||
    [session.error, data.error, audit.error].some(
      (e) => e instanceof ApiError && e.status === 401,
    )
  )
    return <Login done={reset} mode={mode.data} expired={sessionExpired} />;
  if (!session.data) return <Failure retry={() => void session.refetch()} />;
  const canSeeMockPayments =
    mode.data === "MOCK" &&
    session.data.roles.some((r) =>
      ["ADMIN", "OPERATOR", "VIEWER", "APPROVER", "AUDITOR"].includes(r),
    );
  const canLookupPix =
    session.data.roles.some((r) =>
      ["ADMIN", "VIEWER", "OPERATOR"].includes(r),
    ) &&
    (mode.data === "MOCK" ||
      (data.data &&
        data.data.source !== "MOCK" &&
        data.data.pix_lookup_enabled));
  return (
    <div className={"shell " + (dark ? "dark" : "")}>
      <a className="skip-link" href="#conteudo">Ir para o conteúdo</a>
      <aside>
        <Brand />
        <button className="menu-toggle" aria-expanded={menuOpen} aria-controls="navegacao" onClick={() => setMenuOpen(!menuOpen)}>{menuOpen ? <X size={20} /> : <Menu size={20} />} {menuOpen ? "Fechar menu" : "Menu"}</button>
        <div className="navgroup">ÁREA FINANCEIRA</div>
        <nav id="navegacao" aria-label="Navegação principal" className={menuOpen ? "open" : ""}>
          {[
            { name: "Visão geral", icon: LayoutDashboard },
            { name: "Extrato", icon: ListFilter },
            ...(canLookupPix
              ? [{ name: "Cobranças Pix", icon: ScanLine }]
              : []),
            ...(session.data.roles.some((r) =>
              ["ADMIN", "OPERATOR", "VIEWER", "AUDITOR"].includes(r),
            )
              ? [{ name: "Recebimentos", icon: Receipt }]
              : []),
            ...(canSeeMockPayments
              ? [{ name: "Pagamentos Pix", icon: Wallet }]
              : []),
            ...(mode.data === "MOCK"
              ? [
                  { name: "Contas MOCK", icon: CalendarDays },
                  { name: "Pix Automático", icon: Repeat },
                ]
              : []),
            ...(canAudit ? [{ name: "Auditoria", icon: ShieldCheck }] : []),
          ].map(({ name, icon: Icon }) => (
            <button
              aria-current={view === name ? "page" : undefined}
              key={name}
              onClick={() => {
                setView(name);
                setMenuOpen(false);
                document.getElementById("conteudo")?.focus();
              }}
            >
              <Icon size={18} />
              {name}
            </button>
          ))}
        </nav>
        <div className="asidefoot">
          <div className="badge">{environmentLabel(mode.data)}</div>
          <p>
            {mode.data === "PRODUCTION"
              ? "Dados bancários do Inter"
              : mode.data === "SANDBOX" ? "Homologação bancária" : "Dados demonstrativos fictícios"}
          </p>
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
        <main className="content" id="conteudo" tabIndex={-1}>
          {logout.error && (
            <p role="alert">Falha ao encerrar a sessão. Tente novamente.</p>
          )}
          <div className="pagehead">
            <div>
              <span className="eyebrow">PAINEL FINANCEIRO</span>
              <h1>{view}</h1>
            </div>
            {(view === "Visão geral" || view === "Extrato") &&
              mode.data === "MOCK" && (
                <span className="period">
                  <CalendarDays size={16} /> {data.data?.source === "MOCK" ? new Intl.DateTimeFormat("pt-BR", { month: "long", year: "numeric", timeZone: "UTC" }).format(new Date(data.data.as_of + "T12:00:00Z")) : "Base demonstrativa"}
                </span>
              )}
          </div>
          {view === "Contas MOCK" && mode.data === "MOCK" ? (
            <Financeiro user={session.data} />
          ) : view === "Pix Automático" && mode.data === "MOCK" ? (
            <PixAutomatico user={session.data} />
          ) : view === "Pagamentos Pix" && canSeeMockPayments ? (
            <MockPayments
              key={session.data.company_id + ":" + session.data.id}
              user={session.data}
              onExpired={() => setSessionExpired(true)}
            />
          ) : view === "Recebimentos" ? (
            <Recebimentos
              key={session.data.id + mode.data}
              mode={mode.data}
              user={session.data}
            />
          ) : view === "Cobranças Pix" && canLookupPix ? (
            <Pix
              key={session.data.id + mode.data}
              mode={mode.data}
              scope={session.data.company_id + ":" + session.data.id}
              canIssue={session.data.roles.some(
                (r) => r === "ADMIN" || r === "OPERATOR",
              )}
            />
          ) : view === "Auditoria" ? (
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
          ) : data.data.source !== "MOCK" ? (
            <Banking
              data={data.data}
              admin={session.data.roles.includes("ADMIN")}
              scope={session.data.email + data.data.source}
              ledger={view === "Extrato"}
            />
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
