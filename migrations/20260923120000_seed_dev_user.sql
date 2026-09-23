-- The API has no auth yet: every query is hardcoded to user_id = 1.
-- Without this row, tasks.user_id would violate its foreign key and every
-- POST /api/tasks would fail on a fresh database.
INSERT INTO users (id, email, password_hash)
VALUES (1, 'dev@example.com', '')
ON CONFLICT (id) DO NOTHING;

-- id was handed out explicitly above, so the BIGSERIAL sequence still thinks
-- the next id is 1. Move it past the rows that exist.
SELECT setval(pg_get_serial_sequence('users', 'id'), (SELECT max(id) FROM users));
