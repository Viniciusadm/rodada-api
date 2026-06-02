-- The game catalog moved from the database into code
-- (`domain::games::catalog`). Drop the foreign keys into the catalog tables,
-- then drop the tables themselves. `matches.game_id` / `matches.game_mode_id`
-- and `question_banks.game_id` stay as plain CHAR(36) columns that reference the
-- stable catalog UUIDs defined in code.

ALTER TABLE matches DROP FOREIGN KEY matches_game_fk;
ALTER TABLE matches DROP FOREIGN KEY matches_game_mode_fk;
ALTER TABLE question_banks DROP FOREIGN KEY question_banks_game_fk;

DROP TABLE game_modes;
DROP TABLE games;

-- Auto-scored game kinds (Verdadeiro ou Falso, Complete a Palavra) record the
-- player's submitted answer for grading and audit. NULL for buzzer games, which
-- are judged by the host.
ALTER TABLE match_answers ADD COLUMN player_answer TEXT NULL AFTER result;
