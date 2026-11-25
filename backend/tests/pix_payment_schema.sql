BEGIN;
INSERT INTO companies VALUES ('00000000-0000-0000-0000-000000000001', 'Empresa teste'), ('00000000-0000-0000-0000-000000000002', 'Outra empresa');
INSERT INTO users(id, company_id, email, password_hash) VALUES
 ('10000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000001', 'autor@example.test', 'teste'),
 ('10000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000002', 'outro@example.test', 'teste');
INSERT INTO bank_accounts VALUES ('20000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000001', 'Conta teste');
INSERT INTO pix_payment_intents(id,company_id,account_id,creator_id,amount_cents,recipient_ciphertext,idempotency_key) VALUES
 ('30000000-0000-0000-0000-000000000001','00000000-0000-0000-0000-000000000001','20000000-0000-0000-0000-000000000001','10000000-0000-0000-0000-000000000001',1,'\x01','40000000-0000-0000-0000-000000000001');
DO $$
BEGIN
  BEGIN
    UPDATE pix_payment_intents SET approver_id=creator_id, approved_at=now(), state='APROVADO';
    RAISE EXCEPTION 'Autoaprovação indevidamente aceita';
  EXCEPTION WHEN check_violation THEN NULL; END;
  BEGIN
    UPDATE pix_payment_intents SET approver_id='10000000-0000-0000-0000-000000000002', approved_at=now(), state='APROVADO';
    RAISE EXCEPTION 'Aprovação de outra empresa indevidamente aceita';
  EXCEPTION WHEN foreign_key_violation THEN NULL; END;
  BEGIN
    INSERT INTO pix_payment_intents(id,company_id,account_id,creator_id,amount_cents,recipient_ciphertext,idempotency_key)
    SELECT '30000000-0000-0000-0000-000000000002',company_id,account_id,creator_id,amount_cents,recipient_ciphertext,idempotency_key FROM pix_payment_intents;
    RAISE EXCEPTION 'Idempotência duplicada indevidamente aceita';
  EXCEPTION WHEN unique_violation THEN NULL; END;
  BEGIN
    UPDATE pix_payment_intents SET state='EM_ENVIO';
    RAISE EXCEPTION 'Envio sem aprovação indevidamente aceito';
  EXCEPTION WHEN check_violation THEN NULL; END;
END $$;
ROLLBACK;
