#!/usr/bin/env bash
set -euo pipefail


### 2. Create the dedicated service user and group (if they don't exist)
SERVICE_USER="hivehost_server"
SERVICE_GROUP="hivehost_server"
SFTP_GROUP="sftp_users"
BTRFS_DEV_MOUNT_POINT="/hivehost/dev"
PROD_MOUNT_BASE="/hivehost/prod"
USERS_BASE="/hivehost/users"
TEMP_BASE="/hivehost/temp"
HIVEHOST_BASE="/hivehost"

if [ "$(id -u)" -ne 0 ]; then
  echo "ERROR: Please run this script as root (or use sudo). Exiting."
  exit 1
fi

### 2. Create Service User/Group (Idempotent)
if ! getent group "$SERVICE_GROUP" >/dev/null; then
  echo "Creating group '$SERVICE_GROUP'..."
  groupadd --system "$SERVICE_GROUP"
else
  echo "Group '$SERVICE_GROUP' already exists."
fi

if ! id -u "$SERVICE_USER" >/dev/null 2>&1; then
  echo "Creating user '$SERVICE_USER'..."
  useradd --system --gid "$SERVICE_GROUP" --shell /usr/sbin/nologin --no-create-home "$SERVICE_USER"
else
  echo "User '$SERVICE_USER' already exists."
fi

### 3. Create SFTP Group (Idempotent)
if ! getent group "$SFTP_GROUP" >/dev/null; then
  echo "Creating group '$SFTP_GROUP'..."
  groupadd --system "$SFTP_GROUP"
else
  echo "Group '$SFTP_GROUP' already exists."
fi

echo "Creating base directories..."
mkdir -p "$HIVEHOST_BASE"
chown root:root "$HIVEHOST_BASE"
chmod 755 "$HIVEHOST_BASE" # Or 711 if you prefer

mkdir -p "$BTRFS_DEV_MOUNT_POINT"
chown root:root "$BTRFS_DEV_MOUNT_POINT"
chmod 711 "$BTRFS_DEV_MOUNT_POINT" # Restrict listing

mkdir -p "$PROD_MOUNT_BASE"
chown root:root "$PROD_MOUNT_BASE"
chmod 755 "$PROD_MOUNT_BASE"

mkdir -p "$USERS_BASE"
chown root:root "$USERS_BASE"
chmod 755 "$USERS_BASE" # SFTP Chroot base

mkdir -p "$TEMP_BASE"
chown root:root "$TEMP_BASE"
chmod 755 "$TEMP_BASE" # Temporary files




### 5. Check/Verify ACLs on BTRFS_DEV_MOUNT_POINT
echo "Checking ACL support on '$BTRFS_DEV_MOUNT_POINT'..."
# Ensure the BTRFS filesystem itself is mounted with ACL support in /etc/fstab!
# Example fstab: UUID=... /hivehost/dev btrfs defaults,acl 0 0
if ! mount | grep -q " on $BTRFS_DEV_MOUNT_POINT .*acl"; then
  echo "WARNING: Filesystem at '$BTRFS_DEV_MOUNT_POINT' might not be mounted with ACL support."
  echo "Please ensure 'acl' option is present in /etc/fstab for this mount point."
  # Attempting a remount might work temporarily but isn't persistent
  # mount -o remount,acl "$BTRFS_DEV_MOUNT_POINT" || echo "Remount attempt failed."
fi

# Test ACL setting ability
echo "Testing ACL functionality on '$BTRFS_DEV_MOUNT_POINT'..."
if setfacl -m "u:$SERVICE_USER:rwx" "$BTRFS_DEV_MOUNT_POINT" >/dev/null 2>&1; then
  echo "ACL test successful. Granting service user base access."
  # Grant service user ability to manage items *within* BTRFS_DEV_MOUNT_POINT
  setfacl -m "u:$SERVICE_USER:rwx" "$BTRFS_DEV_MOUNT_POINT"
  setfacl -d -m "u:$SERVICE_USER:rwx" "$BTRFS_DEV_MOUNT_POINT"
  setfacl -m "u:root:rwx" "$BTRFS_DEV_MOUNT_POINT" # Ensure root retains full ACL control
  setfacl -d -m "u:root:rwx" "$BTRFS_DEV_MOUNT_POINT" # Ensure root retains full ACL control
else
  # Cleanup potential failed ACL test if it left an entry
  setfacl -x "u:$SERVICE_USER" "$BTRFS_DEV_MOUNT_POINT" >/dev/null 2>&1 || true
  echo "ERROR: Failed to set test ACL on '$BTRFS_DEV_MOUNT_POINT'."
  echo "Ensure the filesystem is mounted with 'acl' option and supports ACLs."
  exit 1 # ACLs are critical for this design
fi

# Grant service user access to manage production mount points
echo "Granting service user access to '$PROD_MOUNT_BASE'..."
setfacl -m "u:$SERVICE_USER:rwx" "$PROD_MOUNT_BASE"
setfacl -d -m "u:$SERVICE_USER:rwx" "$PROD_MOUNT_BASE" # Allow creating mount dirs

# Grant service user access to manage temporary files
echo "Granting service user access to '$TEMP_BASE'..."
setfacl -m "u:$SERVICE_USER:rwx" "$TEMP_BASE"
setfacl -d -m "u:$SERVICE_USER:rwx" "$TEMP_BASE" # Allow creating temp files






echo "✅ Initialization Script Completed."
echo "Base directories created under '$HIVEHOST_BASE'."
echo "Service user '$SERVICE_USER' and SFTP group '$SFTP_GROUP' ensured."
echo "ACL support verified on '$BTRFS_DEV_MOUNT_POINT'."
echo "Base permissions set for '$SERVICE_USER'."
echo "---"
echo "Next Steps:"
echo "1. Ensure your Btrfs volume is mounted at '$BTRFS_DEV_MOUNT_POINT' with 'acl' in /etc/fstab."
echo "2. Configure SSHD for SFTP Chroot (Match Group $SFTP_GROUP, ChrootDirectory $USERS_BASE/%u, ForceCommand internal-sftp, etc.)."
echo "3. Ensure your backend service runs as/uses '$SERVICE_USER' (likely via sudo for helper commands)."
echo "4. Ensure /etc/hivehost/.env exist and /usr/local/bin/hivehost-ssh-keys.sh is 755 owned by root"

exit 0
