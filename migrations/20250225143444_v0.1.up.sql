-- Add up migration script here
create table Jobs (
    id integer not null,
    flake text not null,
    custom_name text, -- 
    finished date, -- When evaluating was done
    timeTookSecs int, -- How long evaluating took
    state int, -- Done, Evaluating, Building, etc..
    logs text, -- is needed if evaluation fails

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
