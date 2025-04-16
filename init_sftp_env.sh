#!/usr/bin/env bash
set -euo pipefail

# Ce script doit s’exécuter en root ou via sudo.

### 1. Vérifier si on est root
if [ "$(id -u)" -ne 0 ]; then
  echo "Please run as root (or use sudo). Exiting."
  exit 1
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

### 4. (Optionnel) Activer l'ACL sur /projects si ce n'est pas déjà monté avec ACL
# Sur Ubuntu/Debian, on peut le faire en éditant /etc/fstab
#   (ex: "/dev/sda1 /projects ext4 defaults,acl 0 2")
# Puis : mount -o remount,acl /projects
# Vérification :
if ! mount | grep -q " on /projects " ; then
  echo "Warning: /projects might not be a separate partition. ACL activation might need /etc/fstab config."
fi

# Test si ACL fonctionne :
setfacl -m u:root:rwx /projects 2>/dev/null || {
  echo "ACL might not be enabled on /projects (setfacl command failed)."
  echo "Please ensure the filesystem is mounted with 'acl' option."
}

echo "✅ Initialization done."
echo "Folders created and permissions set:
 - /sftp/users
 - /projects
"

exit 0