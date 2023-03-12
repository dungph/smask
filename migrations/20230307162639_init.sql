create table smask_cell (
    smask_cell_id integer primary key,
    smask_cell_table text not null,
    smask_cell_column text not null,
    smask_cell_data blob not null
);

create table smask_role (
    smask_role_key blob primary key,
    smask_role_name text not null
);

create table smask_role_table (
    smask_role_key blob not null,
    smask_table_name text not null
);

create table smask_role_column (
    smask_role_key blob not null,
    smask_table_name text not null,
    smask_column_name text not null
);
