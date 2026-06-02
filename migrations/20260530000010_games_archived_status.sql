-- Soft-delete for catalog games: an archived game is hidden from the public
-- catalog but kept for history and admin listings.
ALTER TABLE games DROP CHECK games_status_check;
ALTER TABLE games
  ADD CONSTRAINT games_status_check CHECK (status IN ('active', 'beta', 'maintenance', 'coming_soon', 'archived'));
