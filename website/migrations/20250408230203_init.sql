create type role_type as enum ('admin', 'user');
create type permission_type as enum ('read', 'write', 'owner');

create table if not exists users
(
    id       bigserial primary key,
    username    text not null,
    email text not null unique,
    password text not null,
    role role_type not null
);



create table if not exists projects
(
    id BIGSERIAL primary key,
    name text not null
);





create table if not exists permissions
(
    user_id     BIGINT references users (id) on delete cascade,
    project_id  BIGINT references projects (id) on delete cascade,
    permission  permission_type not null,
    primary key(user_id, project_id)
);