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

create table Jobsets (
    id integer not null,
    project_id int not null,
    flake text not null,
    name varchar(255) not null,
    description varchar(255) not null,
    last_evaluated date,
    last_checked date,
    check_interval int not null,
    evaluation_took int, -- seconds
    state text, -- eval_running, idle

    primary key (id),
    foreign key (project_id) references Projects(id)
);

create table Projects (
    id integer not null,
    name varchar(255) not null,
    description varchar(255) not null,

    primary key (id)
);
