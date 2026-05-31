ALTER TABLE matches
  ADD CONSTRAINT matches_current_round_fk FOREIGN KEY (current_round_id) REFERENCES match_rounds (id),
  ADD CONSTRAINT matches_winner_participant_fk FOREIGN KEY (winner_participant_id) REFERENCES match_participants (id);
