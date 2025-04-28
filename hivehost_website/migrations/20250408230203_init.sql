create type role_type as enum ('admin', 'user');
create type permission_type as enum ('read', 'write', 'owner');

create table if not exists users
(
    id       bigserial primary key,
    username text      not null,
    email    text      not null unique,
    password text      not null,
    role     role_type not null,
    slug     TEXT
        GENERATED ALWAYS AS (username || id::TEXT)
            STORED not null
);



create table if not exists projects
(
    id        bigserial primary key,
    name      text                  not null,
    is_active boolean default false not null,
    slug      TEXT
        GENERATED ALWAYS AS (name || id::TEXT)
            STORED not null
);



create table if not exists permissions
(
    user_id    BIGINT references users (id) on delete cascade    NOT NULL,
    project_id BIGINT references projects (id) on delete cascade NOT NULL,
    permission permission_type                                   not null,
    primary key (user_id, project_id)
);

create table if not exists user_ssh_keys
(
    id         bigserial primary key,
    name       text                                           not null,
    user_id    BIGINT references users (id) on delete cascade NOT NULL,
    public_key text                                           not null
);