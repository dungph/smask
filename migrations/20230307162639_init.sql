create table smask_table (
    smask_table_id integer primary key,
    smask_table_name text not null
);
create table smask_column (
    smask_column_id integer primary key,
    smask_table_id integer not null,
    smask_column_name text not null,
    smask_column_encrypted integer not null
);
create table smask_record (
    smask_record_id integer primary key,
    smask_table_id integer not null,
    smask_record_encrypted integer not null
);
create table smask_cell (
    smask_cell_id integer primary key,
    smask_table_id integer not null,
    smask_record_id integer not null,
    smask_column_id integer not null,
    smask_cell_value blob not null
);
create table smask_role (
    smask_role_id integer primary key,
    smask_role_name text not null,
    smask_role_pubkey blob not null
);
create table smask_role_table (
    smask_role_table_id integer primary key,
    smask_role_id integer not null,
    smask_table_id integer not null
);
create table smask_role_column (
    smask_role_column_id integer primary key,
    smask_role_id integer not null,
    smask_column_id integer not null
);
create table smask_role_cell (
    smask_role_cell_id integer primary key,
    smask_role_id integer not null,
    smask_cell_id integer not null
);
