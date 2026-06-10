CREATE TABLE games (
  id CHAR(36) NOT NULL,
  slug VARCHAR(120) NOT NULL,
  name VARCHAR(140) NOT NULL,
  description TEXT NOT NULL,
  status VARCHAR(40) NOT NULL,
  icon_key VARCHAR(80) NULL,
  sort_order INT NOT NULL DEFAULT 0,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY games_slug_unique (slug),
  KEY games_status_sort_idx (status, sort_order),
  CONSTRAINT games_status_check CHECK (status IN ('active', 'beta', 'maintenance', 'coming_soon'))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE game_modes (
  id CHAR(36) NOT NULL,
  game_id CHAR(36) NOT NULL,
  code VARCHAR(60) NOT NULL,
  name VARCHAR(120) NOT NULL,
  description TEXT NULL,
  default_initial_points INT NULL,
  default_time_per_question_seconds INT NULL,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY game_modes_game_code_unique (game_id, code),
  KEY game_modes_game_active_idx (game_id, is_active),
  CONSTRAINT game_modes_game_fk FOREIGN KEY (game_id) REFERENCES games (id),
  CONSTRAINT game_modes_code_check CHECK (code IN ('classic', 'initial_points', 'elimination')),
  CONSTRAINT game_modes_default_initial_points_check CHECK (default_initial_points IS NULL OR default_initial_points >= 0),
  CONSTRAINT game_modes_default_time_check CHECK (default_time_per_question_seconds IS NULL OR default_time_per_question_seconds > 0)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
