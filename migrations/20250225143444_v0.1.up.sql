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
    error_message text, -- Eval error messages

    primary key (id),
    foreign key (project_id) references Projects(id)
);

create table Projects (
    id integer not null,
    name varchar(255) not null,
    description varchar(255) not null,

    primary key (id)
);

create table Evaluations (
    id integer not null,
    jobset_id int not null,

    primary key (id),
    foreign key (jobset_id) references Jobsets(id)
);

create table Jobs (
    id integer not null,
    evaluation_id int not null,
    attribute_name text not null, -- name of attribute in hydraJobs. like: "systems.main" or "systems.wattson"
    derivation_path text not null,


    primary key (id)
    foreign key (evaluation_id) references Evaluations(id)
)
