create table smask_role (
    smask_key blob primary key,
    smask_role text not null
);

create table smask_role_table (
    smask_key blob not null,
    smask_table text not null
);

create table smask_role_column (
    smask_key blob not null,
    smask_table text not null,
    smask_column text not null
);

create table smask_encrypted (
    smask_column text not null,
    smask_table text not null
);
