CREATE TABLE match_templates (
  id CHAR(36) NOT NULL,
  host_user_id CHAR(36) NOT NULL,
  name VARCHAR(180) NOT NULL,
  description TEXT NULL,
  mode VARCHAR(20) NOT NULL,
  scoring_mode VARCHAR(20) NOT NULL DEFAULT 'classic',
  max_rounds INT NULL,
  time_per_round_seconds INT NULL,
  initial_points INT NOT NULL DEFAULT 0,
  remote_enabled BOOLEAN NOT NULL DEFAULT TRUE,
  capacity INT NULL,
  buzzer_arm_delay_seconds INT NOT NULL DEFAULT 0,
  ranking_display VARCHAR(20) NOT NULL DEFAULT 'always',
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY match_templates_host_created_idx (host_user_id, created_at),
  CONSTRAINT match_templates_host_fk FOREIGN KEY (host_user_id) REFERENCES users (id),
  CONSTRAINT match_templates_mode_check CHECK (mode IN ('buzzer', 'quiz')),
  CONSTRAINT match_templates_scoring_mode_check CHECK (scoring_mode IN ('classic', 'initial_points', 'elimination')),
  CONSTRAINT match_templates_ranking_display_check CHECK (ranking_display IN ('always', 'between_questions', 'end_only')),
  CONSTRAINT match_templates_max_rounds_check CHECK (max_rounds IS NULL OR max_rounds > 0),
  CONSTRAINT match_templates_time_check CHECK (time_per_round_seconds IS NULL OR time_per_round_seconds > 0),
  CONSTRAINT match_templates_initial_points_check CHECK (initial_points >= 0),
  CONSTRAINT match_templates_capacity_check CHECK (capacity IS NULL OR capacity > 0)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE match_template_questions (
  id CHAR(36) NOT NULL,
  template_id CHAR(36) NOT NULL,
  source_question_id CHAR(36) NULL,
  position INT NOT NULL,
  question_type VARCHAR(20) NOT NULL DEFAULT 'free_text',
  prompt TEXT NOT NULL,
  expected_answer TEXT NOT NULL,
  options TEXT NULL,
  points INT NOT NULL DEFAULT 100,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY match_template_questions_template_position_unique (template_id, position),
  KEY match_template_questions_source_question_idx (source_question_id),
  CONSTRAINT match_template_questions_template_fk FOREIGN KEY (template_id) REFERENCES match_templates (id) ON DELETE CASCADE,
  CONSTRAINT match_template_questions_source_question_fk FOREIGN KEY (source_question_id) REFERENCES questions (id) ON DELETE SET NULL,
  CONSTRAINT match_template_questions_position_check CHECK (position > 0),
  CONSTRAINT match_template_questions_type_check CHECK (question_type IN ('buzzer', 'true_false', 'free_text', 'multiple_choice')),
  CONSTRAINT match_template_questions_points_check CHECK (points >= 0)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

ALTER TABLE matches
  ADD COLUMN template_id CHAR(36) NULL AFTER mode,
  ADD KEY matches_template_idx (template_id),
  ADD CONSTRAINT matches_template_fk FOREIGN KEY (template_id) REFERENCES match_templates (id) ON DELETE SET NULL;
