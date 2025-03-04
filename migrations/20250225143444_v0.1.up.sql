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
    state text, -- JobsetState

    primary key (id),
    foreign key (project_id) references Projects(id)
);

create table Projects (
    id integer not null,
    name varchar(255) not null,
    description varchar(255) not null,

    primary key (id)
);

create table Derivations (
    id integer not null,
    jobset_id int not null,
    attribute_name text not null, -- name of attribute in hydraJobs. like: "systems.main" or "systems.wattson"
    derivation_path text not null,


    primary key (id)
    foreign key (jobset_id) references Jobsets(id)
)
