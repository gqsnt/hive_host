# HiveHost 

HiveHost is a platform designed for self-hosting applications, providing a streamlined way to manage and deploy various projects with integrated features for development, collaboration, and serving.

## Overview

The project aims to simplify the complexities typically associated with managing servers, deploying applications, handling permissions, and serving content. It provides a user-friendly web interface for users and a robust backend system for server administration.

## Architecture

HiveHost operates as a **Distributed Multiservice System**. This architecture separates concerns into distinct services, allowing for specialized tasks, privilege separation, and the management of potentially multiple hosting servers from a single central website. The core components include:

1.  **Website (`hivehost_website`):** The user-facing web application built with Leptos and Axum. This service acts as the **central control plane**. It manages user authentication (login, signup, sessions), authorization (permissions), and stores all core application metadata (users, projects, permissions, servers, snapshots, SSH keys) in a **PostgreSQL database**. It serves the web UI and communicates with individual `hivehost_server` instances via Tarpc for actions that need to be performed on a specific hosting machine.
2.  **Server (`hivehost_server`):** This service runs on each individual hosting machine and acts as an **RPC gateway and local task orchestrator**. It receives RPC requests from the `hivehost_website` over a standard TCP connection. The Website service authenticates itself to the Server by sending a pre-shared token via an initial RPC call. The Server delegates system-level operations to the local `hivehost_server_helper` and triggers static content management tasks on the local `hivehost_server_hosting` via Tarpc. It also handles the logic for web-based file Browse, editing, uploading, and downloading directly on its machine, ensuring paths are within project scope. It **does not handle user authentication or the primary application database itself**.
3.  **Helper (`hivehost_server_helper`):** A privileged background service that runs on each hosting machine. It is responsible for executing system-level commands (like user/group management, ACLs, Btrfs operations, mounts) with necessary privileges. It receives specific instructions from its paired `hivehost_server` via Tarpc.
4.  **Hosting (`hivehost_server_hosting`):** A dedicated service running on each hosting machine, focused on efficiently serving static project content over HTTP. It retrieves initial project data (which projects to serve) from the PostgreSQL database for caching purposes, but does not manage core application metadata. It receives commands from its paired `hivehost_server` via Tarpc to update its cache and manage served projects (e.g., reload after a snapshot is made active).

Inter-service communication primarily utilizes **Tarpc** with **Bincode** serialization. The web interface communicates with the `hivehost_server` through RPC for control actions and dedicated HTTP endpoints for file uploads/downloads handled directly by the Server service using a token mechanism.

## Key Features

Based on recent development efforts and implemented functionality:

* **Centralized User & Project Management:** A single web interface (the Website service) handles all user accounts, project creation across multiple servers, team permissions, and project settings, backed by a central database.
* **Multi-Server Hosting:** Projects can be created and managed on different connected hosting servers.
* **Advanced File Management:** Provides secure file access via SFTP (leveraging system users and permissions managed by the Helper service, enabling secure environments such as Chroot configuration). Additionally, it offers comprehensive web-based file management features directly through the web interface. Users can browse directories, create, rename, and delete files and folders, and perform web-based file viewing/editing, uploading multiple files, and downloading single files. These web-based operations are securely mediated by the `hivehost_server` on the target machine using a token-based HTTP mechanism, ensuring path validation within project scope.
* **Btrfs Snapshots:** Create efficient, read-only Btrfs snapshots of project development environments for backups and rollback. Manage existing snapshots (list, delete, restore to a previous state).
* **Production Deployment:** Easily designate a specific project snapshot to be served as the live production version via the Hosting service, with seamless switching and the ability to unset the active version.
* **Granular Team Permissions:** Invite and manage team members for each project, assigning specific permissions (Read, Write, Owner) enforced by the Website service (using database metadata) and propagated to the hosting server (using system ACLs via the Helper).
* **Secure Access:** Utilizes SSH keys for SFTP access, enabling secure environments (such as Chroot) through the system user and permission management performed by the Helper service. It also employs a token-based system for secure web-based file operations, integrated with database-backed user authentication, system ACLs, and CSRF protection for web actions.
* **Robustness:** Custom error types and input validation are implemented across services. RPC clients include auto-reconnection logic for reliability.

## Technologies

HiveHost is primarily developed in **Rust**, leveraging its performance and safety features. Key technologies and concepts used include:

* **Rust:** The core programming language.
* **Distributed Multiservice Architecture:** System composed of specialized, communicating services.
* **Tarpc:** For inter-service RPC (Website to Server, Server to Helper/Hosting) using Bincode.
* **Leptos / Axum:** The frontend framework and web server for the central Website service.
* **Hyper:** The HTTP engine used by the Hosting service for serving static content.
* **PostgreSQL / Sqlx:** The central database for application metadata, accessed primarily by the Website service.
* **Btrfs:** Utilized by the Helper service for filesystem management (subvolumes, snapshots, mounts).
* **ACLs:** System Access Control Lists, managed by the Helper, for file permissions.
* **Systemd:** For managing backend service processes.
* **SSH / SFTP:** Provides secure file transfer access, integrated with system user management by the Helper.
* **Axum Sessions & Authentication:** Frameworks used in the Website service for user login and session management.
* **Input Validation:** Using the `validator` crate.
* **Error Handling:** Structured custom error types across services.

## Project Status

HiveHost is under active development.