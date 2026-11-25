ALTER TABLE bank_transactions
 ADD COLUMN inter_operation char(1),
 ADD COLUMN inter_title text,
 ADD COLUMN inter_description text,
 ADD COLUMN inter_document_number text,
 ADD COLUMN inter_included_on date;

ALTER TABLE bank_transactions
 ADD CONSTRAINT bank_transactions_inter_operation_check
 CHECK (inter_operation IS NULL OR inter_operation IN ('C', 'D'));
