CREATE TABLE organizations (
  id CHAR(36) NOT NULL,
  name VARCHAR(160) NOT NULL,
  slug VARCHAR(120) NOT NULL,
  contact_email VARCHAR(255) NULL,
  owner_user_id CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY organizations_slug_unique (slug),
  KEY organizations_owner_user_id_idx (owner_user_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE users (
  id CHAR(36) NOT NULL,
  email VARCHAR(255) NOT NULL,
  password_hash VARCHAR(255) NOT NULL,
  display_name VARCHAR(140) NOT NULL,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  last_login_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY users_email_unique (email)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE user_roles (
  id CHAR(36) NOT NULL,
  organization_id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  role VARCHAR(40) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY user_roles_org_user_role_unique (organization_id, user_id, role),
  KEY user_roles_user_id_idx (user_id),
  KEY user_roles_org_role_idx (organization_id, role),
  CONSTRAINT user_roles_org_fk FOREIGN KEY (organization_id) REFERENCES organizations (id),
  CONSTRAINT user_roles_user_fk FOREIGN KEY (user_id) REFERENCES users (id),
  CONSTRAINT user_roles_role_check CHECK (role IN ('owner', 'admin', 'host', 'member'))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE ministries (
  id CHAR(36) NOT NULL,
  organization_id CHAR(36) NOT NULL,
  name VARCHAR(120) NOT NULL,
  slug VARCHAR(120) NOT NULL,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY ministries_org_slug_unique (organization_id, slug),
  KEY ministries_org_active_idx (organization_id, is_active),
  CONSTRAINT ministries_org_fk FOREIGN KEY (organization_id) REFERENCES organizations (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

ALTER TABLE organizations
  ADD CONSTRAINT organizations_owner_user_fk FOREIGN KEY (owner_user_id) REFERENCES users (id);
