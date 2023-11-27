use cliclack::{confirm, input, intro, log, multiselect, select, spinner};
use colored::Colorize;
use local_ip_address::local_ip;
use std::{
    collections::HashMap,
    io::Error,
    path::{Path, PathBuf},
};

const BANNER: &str = r"
   |\_    _ _      _
   /o \  | (_) ___| |__   ___  ___ ___   ___  _ __ __ _
 (_. ||  | | |/ __| '_ \ / _ \/ __/ __| / _ \| '__/ _` |
   /__\  | | | (__| | | |  __/\__ \__ \| (_) | | | (_| |
  )___(  |_|_|\___|_| |_|\___||___/___(_)___/|_|  \__, |
                                                   |___/
";

struct Config {
    profiles: Option<Vec<String>>,
    setup_database: Option<bool>,
    su_password: Option<String>,
    password: Option<String>,
    hostname: Option<String>,
    phone_ip: Option<String>,
    connection_port: Option<u16>,
    pairing_port: Option<u16>,
    pairing_code: Option<u32>,
}

impl Config {
    fn to_env(&self) {
        let current_env_contents = match std::fs::read_to_string(".env") {
            Ok(config) => config,
            Err(_) => String::new(),
        };

        let mut config_values: HashMap<String, String> = HashMap::new();
        for line in current_env_contents.lines() {
            let mut split = line.split('=');
            let key = split.next().unwrap();
            let value = split.next().unwrap();
            config_values.insert(key.to_string(), value.to_string());
        }

        let mut new_config_values: HashMap<String, String> = HashMap::new();

        if let Some(profiles) = &self.profiles {
            new_config_values.insert(
                "COMPOSE_PROFILES".to_string(),
                profiles.join(",").to_string(),
            );
        }

        if let Some(setup_database) = &self.setup_database {
            new_config_values.insert("SETUP_DATABASE".to_string(), setup_database.to_string());
        }

        if let Some(su_password) = &self.su_password {
            new_config_values.insert("SU_PASSWORD".to_string(), su_password.to_string());
        }

        if let Some(password) = &self.password {
            new_config_values.insert("PASSWORD".to_string(), password.to_string());
        }

        if let Some(hostname) = &self.hostname {
            new_config_values.insert("LILA_HOSTNAME".to_string(), hostname.to_string());
        }

        if let Some(phone_ip) = &self.phone_ip {
            new_config_values.insert("PHONE_IP".to_string(), phone_ip.to_string());
        }

        if let Some(connection_port) = &self.connection_port {
            new_config_values.insert("CONNECTION_PORT".to_string(), connection_port.to_string());
        }

        if let Some(pairing_port) = &self.pairing_port {
            new_config_values.insert("PAIRING_PORT".to_string(), pairing_port.to_string());
        }

        if let Some(pairing_code) = &self.pairing_code {
            new_config_values.insert("PAIRING_CODE".to_string(), pairing_code.to_string());
        }

        config_values.extend(new_config_values);

        std::fs::write(
            ".env",
            config_values
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<String>>()
                .join("\n"),
        )
        .unwrap();
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
struct Repository {
    org: String,
    project: String,
}

impl Repository {
    fn new(org: &str, project: &str) -> Self {
        Self {
            org: org.to_string(),
            project: project.to_string(),
        }
    }

    fn full_name(&self) -> String {
        format!("{}/{}", self.org, self.project)
    }

    fn url(&self) -> String {
        format!("https://github.com/{}/{}", self.org, self.project)
    }

    fn clone_path(&self) -> PathBuf {
        Path::new("repos").join(&self.project)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository() {
        let repo = Repository::new("lichess-org", "lila");
        assert_eq!(repo.org, "lichess-org");
        assert_eq!(repo.project, "lila");
        assert_eq!(repo.full_name(), "lichess-org/lila");
        assert_eq!(repo.url(), "https://github.com/lichess-org/lila");
        assert_eq!(repo.clone_path(), Path::new("repos/lila"));
    }
}

#[derive(Default, Clone, Eq, PartialEq, Debug)]
struct OptionalService<'a> {
    compose_profile: Option<Vec<&'a str>>,
    repositories: Option<Vec<Repository>>,
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    assert!(args.len() > 1, "Missing command");

    match args[1].as_str() {
        "setup" => setup(),
        "hostname" => hostname(),
        "mobile" => mobile_setup(),
        "gitpod-welcome" => {
            gitpod_welcome();
            Ok(())
        }
        _ => panic!("Unknown command"),
    }
}

fn setup() -> std::io::Result<()> {
    intro(BANNER)?;

    let services = prompt_for_optional_services()?;

    let setup_database =
        confirm("Do you want to seed the database with test users, games, etc? (Recommended)")
            .initial_value(true)
            .interact()?;

    let (su_password, password) = if setup_database {
        (
            input("Choose a password for admin users (blank for 'password')")
                .placeholder("password")
                .default_input("password")
                .required(false)
                .interact()?,
            input("Choose a password for regular users (blank for 'password')")
                .placeholder("password")
                .default_input("password")
                .required(false)
                .interact()?,
        )
    } else {
        (String::new(), String::new())
    };

    Config {
        profiles: Some(
            services
                .iter()
                .filter_map(|service| service.compose_profile.clone())
                .flatten()
                .map(std::string::ToString::to_string)
                .collect(),
        ),
        setup_database: Some(setup_database),
        su_password: Some(su_password),
        password: Some(password),
        hostname: None,
        phone_ip: None,
        connection_port: None,
        pairing_port: None,
        pairing_code: None,
    }
    .to_env();

    create_placeholder_dirs();

    let mut repos_to_clone: Vec<Repository> = vec![
        Repository::new("lichess-org", "lila"),
        Repository::new("lichess-org", "lila-ws"),
        Repository::new("lichess-org", "lila-db-seed"),
        Repository::new("lichess-org", "lifat"),
    ];

    let optional_repos: Vec<Repository> = services
        .iter()
        .filter_map(|service| service.repositories.clone())
        .flatten()
        .collect();

    repos_to_clone.extend(optional_repos);

    for repo in repos_to_clone {
        let mut progress = spinner();
        progress.start(&format!("Cloning {}...", repo.full_name()));

        if repo.clone_path().read_dir()?.next().is_some() {
            progress.stop(format!("Clone {} ✓", repo.full_name()));
            continue;
        }

        let mut cmd = std::process::Command::new("git");
        cmd.arg("clone")
            .arg("--origin")
            .arg("upstream")
            .arg("--depth")
            .arg("1")
            .arg("--recurse-submodules")
            .arg(repo.url())
            .arg(repo.clone_path());

        let output = cmd.output().unwrap();
        assert!(
            output.status.success(),
            "Failed to clone repo: {} - {:?}",
            repo.full_name(),
            output
        );

        progress.stop(format!("Clone {} ✓", repo.full_name()));
    }

    log::success("Wrote .env")
}

fn create_placeholder_dirs() {
    // Create a placeholder directory for each of the repos
    // otherwise the directories will be created by Docker
    // when the volumes are mounted and they may be owned by root
    [
        Repository::new("lichess-org", "lila"),
        Repository::new("lichess-org", "lila-ws"),
        Repository::new("lichess-org", "lila-db-seed"),
        Repository::new("lichess-org", "lifat"),
        Repository::new("lichess-org", "lila-fishnet"),
        Repository::new("lichess-org", "lila-engine"),
        Repository::new("lichess-org", "lila-search"),
        Repository::new("lichess-org", "lila-gif"),
        Repository::new("lichess-org", "api"),
        Repository::new("lichess-org", "chessground"),
        Repository::new("lichess-org", "pgn-viewer"),
        Repository::new("lichess-org", "scalachess"),
        Repository::new("lichess-org", "mobile"),
        Repository::new("lichess-org", "dartchess"),
        Repository::new("lichess-org", "berserk"),
        Repository::new("cyanfish", "bbpPairings"),
    ]
    .iter()
    .map(Repository::clone_path)
    .for_each(|path| {
        std::fs::create_dir_all(path).unwrap();
    });
}

#[allow(clippy::too_many_lines)]
fn prompt_for_optional_services() -> Result<Vec<OptionalService<'static>>, Error> {
    multiselect(
        "Select which optional services to include:\n    (Use arrows, <space> to toggle, <enter> to continue)\n",
    )
    .required(false)
    .item(
        OptionalService {
            compose_profile: vec!["stockfish-play"].into(),
            repositories: vec![Repository::new("lichess-org", "lila-fishnet")].into(),
        },
        "Stockfish Play",
        "for playing against the computer",
    )
    .item(
        OptionalService {
            compose_profile: vec!["stockfish-analysis"].into(),
            repositories: None,
        },
        "Stockfish Analysis",
        "for requesting computer analysis of games",
    )
    .item(
        OptionalService {
            compose_profile: vec!["external-engine"].into(),
            repositories: vec![Repository::new("lichess-org", "lila-engine")].into(),
        },
        "External Engine",
        "for connecting a local chess engine to the analysis board",
    )
    .item(
        OptionalService {
            compose_profile: vec!["search"].into(),
            repositories: vec![Repository::new("lichess-org", "lila-search")].into(),
        },
        "Search",
        "for searching games, forum posts, etc",
    )
    .item(
        OptionalService {
            compose_profile: vec!["gifs"].into(),
            repositories: vec![Repository::new("lichess-org", "lila-gif")].into(),
        },
        "GIFs",
        "for generating animated GIFs of games",
    )
    .item(
        OptionalService {
            compose_profile: vec!["thumbnails"].into(),
            repositories: None,
        },
        "Image uploads + thumbnails",
        "for blog/coach/streamer images",
    )
    .item(
        OptionalService {
            compose_profile: vec!["api-docs"].into(),
            repositories: vec![Repository::new("lichess-org", "api")].into(),
        },
        "API docs",
        "standalone API documentation",
    )
    .item(
        OptionalService {
            compose_profile: vec!["chessground"].into(),
            repositories: vec![Repository::new("lichess-org", "chessground")].into(),
        },
        "Chessground",
        "standalone board UI",
    )
    .item(
        OptionalService {
            compose_profile: vec!["pgn-viewer"].into(),
            repositories: vec![Repository::new("lichess-org", "pgn-viewer")].into(),
        },
        "PGN Viewer",
        "standalone PGN viewer",
    )
    .item(
        OptionalService {
            compose_profile: None,
            repositories: vec![Repository::new("lichess-org", "scalachess")].into(),
        },
        "Scalachess",
        "standalone chess logic library",
    )
    .item(
        OptionalService {
            compose_profile: vec!["mobile"].into(),
            repositories: vec![Repository::new("lichess-org", "mobile")].into(),
        },
        "Mobile app",
        "Flutter-based mobile app",
    )
    .item(
        OptionalService {
            compose_profile: None,
            repositories: vec![Repository::new("lichess-org", "dartchess")].into(),
        },
        "Dartchess",
        "standalone chess library for mobile platforms",
    )
    .item(
        OptionalService {
            compose_profile: None,
            repositories: vec![Repository::new("lichess-org", "berserk")].into(),
        },
        "Berserk",
        "Python API client",
    )
    .item(
        OptionalService {
            compose_profile: None,
            repositories: vec![Repository::new("cyanfish", "bbpPairings")].into(),
        },
        "Swiss Pairings",
        "bbpPairings tool",
    )
    .interact()
}

fn hostname() -> std::io::Result<()> {
    let local_ip = match local_ip() {
        Ok(ip) => ip.to_string(),
        _ => "127.0.0.1".to_string(),
    };

    let hostname: String = match select("Select a hostname to access your local Lichess instance:")
        .initial_value("localhost")
        .item("localhost", "localhost", "default")
        .item(
            "10.0.2.2",
            "10.0.2.2",
            "For accessing from an Android emulator running on this machine",
        )
        .item(
            local_ip.as_str(),
            local_ip.as_str(),
            "Your private IP address, for accessing from other devices on your local network",
        )
        .item("other", "Other", "Enter a custom hostname")
        .interact()?
    {
        "other" => input("Enter a custom hostname:  (It must be resolvable)").interact()?,
        selection => selection.to_string(),
    };

    Config {
        profiles: None,
        setup_database: None,
        su_password: None,
        password: None,
        hostname: Some(hostname),
        phone_ip: None,
        connection_port: None,
        pairing_port: None,
        pairing_code: None,
    }
    .to_env();

    Ok(())
}

fn mobile_setup() -> std::io::Result<()> {
    let phone_ip: String = input("Your phone's private IP address")
        .placeholder("192.168.x.x or 10.x.x.x")
        .interact()?;
    let connection_port: u16 = input("Wireless debugging port")
        .validate(|input: &String| validate_string_length(input, 5))
        .interact()?;
    let pairing_port: u16 = input("Pairing port")
        .validate(|input: &String| validate_string_length(input, 5))
        .interact()?;
    let pairing_code: u32 = input("Pairing code")
        .validate(|input: &String| validate_string_length(input, 6))
        .interact()?;

    Config {
        profiles: None,
        setup_database: None,
        su_password: None,
        password: None,
        hostname: None,
        phone_ip: Some(phone_ip),
        connection_port: Some(connection_port),
        pairing_port: Some(pairing_port),
        pairing_code: Some(pairing_code),
    }
    .to_env();

    Ok(())
}

fn validate_string_length(input: &String, length: usize) -> Result<(), String> {
    match input.len() {
        len if len == length => Ok(()),
        _ => Err(format!("Value should be {length} digits in length")),
    }
}

fn gitpod_welcome() {
    println!("{}", "################".green());
    println!(
        "{}",
        "Your Lichess development environment is starting!".green()
    );
    println!(
        "{}",
        "Monitor the progress in the 'lila' container with the command:".green()
    );
    println!("{}", " docker compose logs lila --follow".green().bold());
    println!(
        "{}",
        "For full documentation, see: https://lichess-org.github.io/lila-gitpod/".green()
    );
}
