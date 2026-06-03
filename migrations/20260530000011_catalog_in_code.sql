ALTER TABLE question_banks DROP FOREIGN KEY question_banks_game_fk;

DROP TABLE game_modes;
DROP TABLE games;

ALTER TABLE match_answers ADD COLUMN player_answer TEXT NULL AFTER result;
