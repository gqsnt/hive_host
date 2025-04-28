#!/bin/bash
set -euo pipefail

# Path: /usr/local/bin/hivehost-ssh-keys.sh
# Purpose: Fetch authorized SSH public keys for a given username from the database.
# Usage: Called by sshd via AuthorizedKeysCommand %u
# Owner: root:root
# Permissions: 755 (rwxr-xr-x)

# --- Configuration ---
# IMPORTANT: Securely manage database credentials!
# Option 1: Use peer authentication if possible.
# Option 2: Read from a config file readable only by root and AuthorizedKeysCommandUser
DB_ENV="/etc/hivehost/.env"
 if [[ -f "$DB_ENV" ]]; then
   source "$DB_ENV" # Ensure this file sets DB_USER, DB_PASS, DB_NAME, DB_HOST, DB_PORT
 else
   echo "Error: DB config $DB_ENV not found" >&2
   exit 1
 fi

DB_USER="${DB_USER:-hivehost_ro}" # A read-only database user is recommended
DB_PASS_VAR="DB_PASSWORD" # Name of env var storing the password (set for AuthorizedKeysCommandUser)
DB_NAME="${DB_NAME:-hivehost_db}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"

# Input validation
if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <username>" >&2
  exit 1
fi
TARGET_USERNAME="$1"

# Basic sanitization (though sshd provides %u) - paranoid check
if [[ ! "$TARGET_USERNAME" =~ ^[a-zA-Z0-9_.-]+$ ]]; then
    echo "Invalid username format." >&2
    exit 1
fi

# Construct the SQL query safely using parameter binding if possible, or careful quoting.
# Execute the query using psql (PostgreSQL example)
# -q: quiet (no informational messages)
# -t: tuples only (no headers/footers)
# -A: unaligned (plain text output, no '|' separators)
# -v target_user="$TARGET_USERNAME": Pass username securely as a variable
# Note: PGPASSWORD environment variable is one way to pass the password securely.
export PGPASSWORD="${!DB_PASS_VAR:-}"
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" \
     -q -t -A \
     -v target_user="$TARGET_USERNAME" <<EOF
SELECT k.public_key
FROM user_ssh_keys k
JOIN users u ON k.user_id = u.id
WHERE u.slug = :'target_user';
EOF

# Unset PGPASSWORD immediately after use
unset PGPASSWORD
# Exit status of psql is passed through. sshd uses this.
exit $?