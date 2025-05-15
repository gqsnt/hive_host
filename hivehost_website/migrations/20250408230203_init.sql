create type role_type as enum ('admin', 'user');
create type permission_type as enum ('read', 'write', 'owner');
create type log_type as enum ('user_action', 'permission_action','snapshot_action', 'io_action', 'token_action');

create table if not exists users
(
    id       bigserial primary key,
    username text      not null,
    email    text      not null unique,
    password text      not null,
    role     role_type not null,
    slug     TEXT
        GENERATED ALWAYS AS (username || '-' || id::TEXT)
            STORED     not null
);

create table if not exists servers
(
    id              bigserial primary key,
    name            text not null,
    ip              text not null,
    hosting_address text not null,
    token           text not null
);


create table if not exists user_githubs
(
    id              bigserial primary key,
    installation_id BIGINT                                         not null,
    user_id         BIGINT references users (id) on delete cascade NOT NULL,
    login           text                                           not null,
    avatar_url      text                                           not null,
    html_url        text                                           not null,
    suspended       boolean default false                          not null
);

create table if not exists projects_github
(
    id              bigserial primary key,
    user_githubs_id BIGINT references user_githubs (id) on delete cascade,
    repo_full_name  text                  not null,
    branch_name     text                  not null,
    dev_commit      text                  not null,
    last_commit     text                  not null,
    auto_deploy     boolean default false not null
);



create table if not exists projects
(
    id                bigserial primary key,
    server_id         BIGINT references servers (id) on delete cascade NOT NULL,
    project_github_id BIGINT references projects_github (id) on delete cascade,
    name              text                                             not null,
    slug              TEXT
        GENERATED ALWAYS AS (name || '-' || id::TEXT)
            STORED                                                     not null
);



create table if not exists projects_snapshots
(
    id            bigserial primary key,
    project_id    BIGINT references projects (id) on delete cascade NOT NULL,
    version       bigint                                            not null,
    snapshot_name text                                              not null,
    name          text,
    description   text,
    git_commit    text,
    git_branch    text,
    created_at    timestamp default now()                           not null
);

alter table projects
    add column if not exists active_snapshot_id BIGINT references projects_snapshots (id) on delete set null;



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


create table if not exists logs
(
    id         bigserial primary key,
    user_id    BIGINT references users (id) on delete cascade NOT NULL,
    project_id BIGINT references projects (id) on delete cascade,
    log_type   log_type                                       not null,
    action     text                                           not null,
    created_at timestamp default now()                        not null
);


INSERT INTO public.servers (id, name, ip, hosting_address, token)
VALUES (1, 'Localhost', '127.0.0.1', 'localhost:3002', 'token_auth_pwd_bbq');


INSERT INTO public.users (id, username, email, password, role)
VALUES (1, 'caribou', 'test@user.com',
        '$argon2id$v=19$m=19456,t=2,p=1$Tib7k2i8Zy0GY5yPd+bh9Q$OcvwV1ThiZCq7oub3Bm0rsUD69d6lKBIaOJt1hmHO/w', 'user');
