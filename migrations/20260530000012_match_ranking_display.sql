ALTER TABLE match_settings
  ADD COLUMN ranking_display VARCHAR(20) NOT NULL DEFAULT 'always',
  ADD CONSTRAINT match_settings_ranking_display_check
    CHECK (ranking_display IN ('always', 'between_questions', 'end_only'));
