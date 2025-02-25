-- Add up migration script here
create table Jobs (
    id integer not null,
    flake text not null,
    custom_name text,
    finished date,
    timeTookSecs int,
    state int,
    logs text,

    primary key (id)
);

create table Derivations (
    id integer not null,
    buildID int not null,
    path text not null,
    output text,

    primary key (id),
    foreign key (buildID) references Jobs(id)
);
