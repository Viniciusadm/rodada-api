ALTER TABLE match_templates
  ADD COLUMN present_screen MEDIUMTEXT NULL AFTER ranking_display;

ALTER TABLE match_settings
  ADD COLUMN present_screen MEDIUMTEXT NULL;
