CREATE TABLE companies (id uuid PRIMARY KEY, name text NOT NULL);
CREATE TABLE users (
 id uuid PRIMARY KEY, company_id uuid NOT NULL REFERENCES companies,
 email text NOT NULL UNIQUE CHECK(email=lower(email)), password_hash text NOT NULL,
 failures integer NOT NULL DEFAULT 0, window_start timestamptz NOT NULL DEFAULT now(),
 active boolean NOT NULL DEFAULT true, UNIQUE(company_id,id)
);
CREATE TYPE user_role AS ENUM ('ADMIN','VIEWER','OPERATOR','APPROVER','AUDITOR');
CREATE TABLE user_roles (user_id uuid REFERENCES users, role user_role, PRIMARY KEY(user_id,role));
CREATE TABLE user_sessions (digest bytea PRIMARY KEY, user_id uuid NOT NULL REFERENCES users, expires_at timestamptz NOT NULL);
CREATE INDEX sessions_by_user ON user_sessions(user_id);
CREATE INDEX session_expiry ON user_sessions(expires_at);
CREATE TABLE bank_accounts (id uuid PRIMARY KEY, company_id uuid NOT NULL REFERENCES companies, label text NOT NULL, UNIQUE(company_id,id));
CREATE TABLE audit_logs (id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY, company_id uuid NOT NULL REFERENCES companies, actor uuid NOT NULL, action text NOT NULL, at timestamptz NOT NULL DEFAULT now(), FOREIGN KEY(company_id,actor) REFERENCES users(company_id,id));
CREATE INDEX audit_company_time ON audit_logs(company_id,at DESC);
