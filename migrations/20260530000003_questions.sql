CREATE TABLE question_banks (
  id CHAR(36) NOT NULL,
  game_id CHAR(36) NULL,
  name VARCHAR(160) NOT NULL,
  description TEXT NULL,
  created_by_user_id CHAR(36) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY question_banks_owner_game_idx (created_by_user_id, game_id),
  CONSTRAINT question_banks_game_fk FOREIGN KEY (game_id) REFERENCES games (id),
  CONSTRAINT question_banks_created_by_fk FOREIGN KEY (created_by_user_id) REFERENCES users (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE questions (
  id CHAR(36) NOT NULL,
  question_bank_id CHAR(36) NOT NULL,
  prompt TEXT NOT NULL,
  expected_answer TEXT NOT NULL,
  default_points INT NOT NULL DEFAULT 100,
  difficulty VARCHAR(40) NULL,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY questions_bank_active_idx (question_bank_id, is_active),
  CONSTRAINT questions_bank_fk FOREIGN KEY (question_bank_id) REFERENCES question_banks (id),
  CONSTRAINT questions_default_points_check CHECK (default_points >= 0),
  CONSTRAINT questions_difficulty_check CHECK (difficulty IS NULL OR difficulty IN ('easy', 'medium', 'hard'))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE content_imports (
  id CHAR(36) NOT NULL,
  question_bank_id CHAR(36) NOT NULL,
  imported_by_user_id CHAR(36) NOT NULL,
  status VARCHAR(40) NOT NULL,
  valid_rows INT UNSIGNED NOT NULL DEFAULT 0,
  invalid_rows INT UNSIGNED NOT NULL DEFAULT 0,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  completed_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  KEY content_imports_bank_created_idx (question_bank_id, created_at),
  KEY content_imports_imported_by_idx (imported_by_user_id),
  CONSTRAINT content_imports_bank_fk FOREIGN KEY (question_bank_id) REFERENCES question_banks (id),
  CONSTRAINT content_imports_imported_by_fk FOREIGN KEY (imported_by_user_id) REFERENCES users (id),
  CONSTRAINT content_imports_status_check CHECK (status IN ('pending', 'completed', 'failed'))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
