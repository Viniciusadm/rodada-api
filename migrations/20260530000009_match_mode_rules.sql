-- Game-mode rules: elimination tracks lives and elimination time per participant.
-- `lives_remaining` is seeded from match_settings.initial_points when an elimination
-- match starts; `eliminated_at` is stamped when a participant runs out of lives.
ALTER TABLE match_participants
  ADD COLUMN lives_remaining INT NULL,
  ADD COLUMN eliminated_at DATETIME(6) NULL,
  ADD CONSTRAINT match_participants_lives_check CHECK (lives_remaining IS NULL OR lives_remaining >= 0);
