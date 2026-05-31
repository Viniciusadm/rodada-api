CREATE TABLE match_participant_tokens (
  id CHAR(36) NOT NULL,
  match_id CHAR(36) NOT NULL,
  participant_id CHAR(36) NOT NULL,
  token VARCHAR(80) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  revoked_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  UNIQUE KEY match_participant_tokens_token_unique (token),
  KEY match_participant_tokens_participant_idx (participant_id),
  KEY match_participant_tokens_match_idx (match_id),
  CONSTRAINT match_participant_tokens_match_fk FOREIGN KEY (match_id) REFERENCES matches (id) ON DELETE CASCADE,
  CONSTRAINT match_participant_tokens_participant_fk FOREIGN KEY (participant_id) REFERENCES match_participants (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
