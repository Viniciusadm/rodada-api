CREATE TABLE room_sessions (
  id CHAR(36) NOT NULL,
  match_id CHAR(36) NOT NULL,
  pin VARCHAR(12) NOT NULL,
  status VARCHAR(40) NOT NULL,
  capacity INT NULL,
  opened_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  closed_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY room_sessions_match_unique (match_id),
  UNIQUE KEY room_sessions_pin_status_unique (pin, status),
  KEY room_sessions_status_opened_idx (status, opened_at),
  CONSTRAINT room_sessions_match_fk FOREIGN KEY (match_id) REFERENCES matches (id) ON DELETE CASCADE,
  CONSTRAINT room_sessions_status_check CHECK (status IN ('waiting', 'live', 'closed')),
  CONSTRAINT room_sessions_capacity_check CHECK (capacity IS NULL OR capacity > 0),
  CONSTRAINT room_sessions_time_order_check CHECK (closed_at IS NULL OR closed_at >= opened_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE presence_sessions (
  id CHAR(36) NOT NULL,
  room_session_id CHAR(36) NOT NULL,
  user_id CHAR(36) NULL,
  participant_id CHAR(36) NULL,
  role VARCHAR(40) NOT NULL,
  connection_id VARCHAR(120) NOT NULL,
  connected_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  last_seen_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  disconnected_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY presence_sessions_connection_unique (connection_id),
  KEY presence_sessions_room_active_idx (room_session_id, disconnected_at),
  KEY presence_sessions_participant_idx (participant_id),
  KEY presence_sessions_user_idx (user_id),
  CONSTRAINT presence_sessions_room_fk FOREIGN KEY (room_session_id) REFERENCES room_sessions (id) ON DELETE CASCADE,
  CONSTRAINT presence_sessions_user_fk FOREIGN KEY (user_id) REFERENCES users (id),
  CONSTRAINT presence_sessions_participant_fk FOREIGN KEY (participant_id) REFERENCES match_participants (id),
  CONSTRAINT presence_sessions_role_check CHECK (role IN ('host', 'presenter', 'player', 'viewer')),
  CONSTRAINT presence_sessions_identity_check CHECK (user_id IS NOT NULL OR participant_id IS NOT NULL),
  CONSTRAINT presence_sessions_time_order_check CHECK (disconnected_at IS NULL OR disconnected_at >= connected_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE buzzer_events (
  id CHAR(36) NOT NULL,
  match_id CHAR(36) NOT NULL,
  match_round_id CHAR(36) NOT NULL,
  participant_id CHAR(36) NOT NULL,
  presence_session_id CHAR(36) NULL,
  decision VARCHAR(40) NOT NULL,
  occurred_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY buzzer_events_round_occurred_idx (match_round_id, occurred_at),
  KEY buzzer_events_participant_round_idx (participant_id, match_round_id),
  KEY buzzer_events_match_idx (match_id),
  KEY buzzer_events_presence_idx (presence_session_id),
  CONSTRAINT buzzer_events_match_fk FOREIGN KEY (match_id) REFERENCES matches (id) ON DELETE CASCADE,
  CONSTRAINT buzzer_events_round_fk FOREIGN KEY (match_round_id) REFERENCES match_rounds (id) ON DELETE CASCADE,
  CONSTRAINT buzzer_events_participant_fk FOREIGN KEY (participant_id) REFERENCES match_participants (id) ON DELETE CASCADE,
  CONSTRAINT buzzer_events_presence_fk FOREIGN KEY (presence_session_id) REFERENCES presence_sessions (id),
  CONSTRAINT buzzer_events_decision_check CHECK (decision IN ('accepted', 'rejected', 'early', 'duplicate'))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE presentation_sessions (
  id CHAR(36) NOT NULL,
  room_session_id CHAR(36) NOT NULL,
  presence_session_id CHAR(36) NULL,
  opened_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  closed_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY presentation_sessions_room_idx (room_session_id),
  KEY presentation_sessions_presence_idx (presence_session_id),
  CONSTRAINT presentation_sessions_room_fk FOREIGN KEY (room_session_id) REFERENCES room_sessions (id) ON DELETE CASCADE,
  CONSTRAINT presentation_sessions_presence_fk FOREIGN KEY (presence_session_id) REFERENCES presence_sessions (id),
  CONSTRAINT presentation_sessions_time_order_check CHECK (closed_at IS NULL OR closed_at >= opened_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE notification_preferences (
  id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  preference_key VARCHAR(80) NOT NULL,
  enabled BOOLEAN NOT NULL DEFAULT TRUE,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY notification_preferences_user_key_unique (user_id, preference_key),
  KEY notification_preferences_user_idx (user_id),
  CONSTRAINT notification_preferences_user_fk FOREIGN KEY (user_id) REFERENCES users (id),
  CONSTRAINT notification_preferences_key_check CHECK (preference_key IN ('new_rooms', 'match_results', 'weekly_summary'))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
