-- Runs once, as root, when the MySQL or MariaDB container first creates its data directory.
-- `crates/nvs-db/src/reset.rs` changes the role kind of session state with `SET ROLE`, which needs a
-- role the matrix's account holds. The role carries no privilege.
CREATE ROLE IF NOT EXISTS novis_reset_role;
GRANT novis_reset_role TO 'novis'@'%';
