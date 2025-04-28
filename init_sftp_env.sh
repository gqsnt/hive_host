#!/usr/bin/env bash
set -euo pipefail

# Ce script doit s’exécuter en root ou via sudo.

### 1. Vérifier si on est root
if [ "$(id -u)" -ne 0 ]; then
  echo "Please run as root (or use sudo). Exiting."
  exit 1
fi


### 2. Create the dedicated service user and group (if they don't exist)
SERVICE_USER="hivehost_server"
SERVICE_GROUP="hivehost_server"
SFTP_GROUP="sftp_users"


if ! getent group "$SERVICE_GROUP" >/dev/null; then
  echo "Creating group '$SERVICE_GROUP'..."
  groupadd --system "$SERVICE_GROUP"
else
  echo "Group '$SERVICE_GROUP' already exists."
fi

if ! id -u "$SERVICE_USER" >/dev/null 2>&1; then
  echo "Creating user '$SERVICE_USER'..."
  # System user, primary group hivehost_server, no login shell, no home dir created by useradd
  useradd --system --gid "$SERVICE_GROUP" --shell /usr/sbin/nologin --no-create-home "$SERVICE_USER"
else
  echo "User '$SERVICE_USER' already exists."
fi

### 3. Create the SFTP group (if it doesn't exist)
if ! getent group "$SFTP_GROUP" >/dev/null; then
  echo "Creating group '$SFTP_GROUP'..."
  groupadd --system "$SFTP_GROUP"
else
  echo "Group '$SFTP_GROUP' already exists."
fi





### 2. Créer le dossier principal sftp
#   owned par root:root, chmod 755
mkdir -p /sftp/users
chown root:root /sftp
chown root:root /sftp/users
chmod 755 /sftp
chmod 755 /sftp/users

### 3. Créer le dossier /projects
#   owned par root:root, chmod 711 (ou 700) pour cacher lister
#   selon tes préférences
mkdir -p /projects
chown root:root /projects
chmod 711 /projects
# 711 => x pour tous => on ne peut pas faire 'ls /projects' si on n'a pas 'r',
# mais on peut cd si on connaît un sous-répertoire.
# tu peux mettre 700 si tu ne veux pas que même 'cd' soit possible.

### 6. Check/Activate ACL on /projects (if needed)
#    (Assuming /projects is a mount point like ext4/xfs supporting ACLs)
if ! mount | grep -q " on /projects .*acl"; then
  echo "Warning: /projects filesystem might not have ACL enabled by default."
  echo "Attempting to remount with ACL..."
  # This might fail if /projects is not a separate mount or already has acl
  mount -o remount,acl /projects || echo "Remount failed, check /etc/fstab or mount options."
fi

# Test if ACL works by trying to set one (and immediately remove it)
echo "Testing ACL functionality..."
if setfacl -m "u:$SERVICE_USER:rwx" /projects >/dev/null 2>&1; then
  echo "ACL test successful. Granting service user initial access."
  # Grant the service user ability to manage files/ACLs *within* /projects
  # It will create subdirs and set specific ACLs later via the helper.
  setfacl -m "u:$SERVICE_USER:rwx" /projects
  setfacl -d -m "u:$SERVICE_USER:rwx" /projects # Default for new items
else
  setfacl -m "u:root:rwx" /projects # Cleanup potential failed attempt
  echo "Error: ACL might not be enabled or working correctly on /projects."
  echo "Please ensure the filesystem for /projects is mounted with the 'acl' option."
  # exit 1 # Optional: exit if ACLs are critical
fi

echo "✅ Initialization done."
echo "User '$SERVICE_USER', Group '$SFTP_GROUP' created (if not existing)."
echo "Folders created and base permissions set:"
echo " - /sftp/users (root:root 755)"
echo " - /projects (root:root 711, +ACL for $SERVICE_USER)"
echo "Ensure your main 'server' application runs as user '$SERVICE_USER'."
echo "Configure SSHD for AuthorizedKeysCommand and Match Group $SFTP_GROUP."

exit 0
