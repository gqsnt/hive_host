#!/bin/bash
set -euo pipefail

# Path: /usr/local/bin/hivehost-ssh-keys.sh
# Purpose: Fetch authorized SSH public keys for a given username from the database.
# Usage: Called by sshd via AuthorizedKeysCommand %u
# Owner: root:root
# Permissions: 755 (rwxr-xr-x)


DB_ENV="/etc/hivehost/.env"
 if [[ -f "$DB_ENV" ]]; then
   source "$DB_ENV" 
 else
   echo "Error: DB config $DB_ENV not found" >&2
   exit 1
 fi

DB_USER="${DB_USER:-hivehost_ro}"
DB_PASS_VAR="DB_PASSWORD" 
DB_NAME="${DB_NAME:-hivehost_db}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"

# Input validation
if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <username>" >&2
  exit 1
fi
TARGET_USERNAME="$1"


if [[ ! "$TARGET_USERNAME" =~ ^[a-zA-Z0-9_.-]+$ ]]; then
    echo "Invalid username format." >&2
    exit 1
fi


export PGPASSWORD="${!DB_PASS_VAR:-}"
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" \
     -q -t -A \
     -v target_user="$TARGET_USERNAME" <<EOF
SELECT k.public_key
FROM user_ssh_keys k
JOIN users u ON k.user_id = u.id
WHERE u.slug = :'target_user';
EOF

unset PGPASSWORD
exit $?