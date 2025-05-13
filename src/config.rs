use std::{
    fs::File,
    io::{stderr, Read},
};

use serde::Deserialize;

// evaluates to
// pub fn get_initial_username(&self) -> String {
//     match &self.initial_username {
//         Some(value) => value.to_string(),
//         None => match &self.initial_username_file {
//             None => panic!("Either initial_username or initial_username_file must be set!"),
//             Some(value) => get_file_content(&value).unwrap(),
//         },
//     }
// }

macro_rules! mk_get_value {
    ($($field:ident),*) => {
        $(
            paste::paste! {
                pub fn [<get_ $field>](&self) -> String {
                    match &self.$field {
                        Some(value) => value.to_string(),
                        None => match &self.[<$field _file>] {
                            None => {
                                eprintln!(
                                "Either {} or {}_file must be set!",
                                stringify!($field),
                                stringify!($field)
                            );
                                std::process::exit(1);
                        },
                            Some(path) => get_file_content(path).unwrap(),
                        }
                    }
                }
            }
        )*
    };
}

fn get_file_content(path: &str) -> Result<String, std::io::Error> {
    let mut file = match File::open(path) {
        Err(e) => return Err(e),
        Ok(file) => file,
    };

    let mut string = String::new();

    match file.read_to_string(&mut string) {
        Err(e) => Err(e),
        Ok(_) => Ok(string),
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct General {
    initial_username: Option<String>,
    initial_username_file: Option<String>,

    initial_password: Option<String>,
    initial_password_file: Option<String>,
}

impl General {
    mk_get_value!(initial_username);
    mk_get_value!(initial_password);
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub general: General,
}

impl Config {
    pub fn parse(path: &str) -> Config {
        let mut file = match File::open(path) {
            Err(e) => panic!("Failed to open config file '{}' because: {}", path, e),
            Ok(file) => file,
        };

        let mut content = String::new();

        match file.read_to_string(&mut content) {
            Err(e) => panic!("Could not read config file content because: {}", e),
            Ok(_) => {
                return toml::from_str(&content).unwrap();
            }
        }
    }
}
