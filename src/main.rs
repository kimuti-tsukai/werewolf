use std::{
    collections::{HashMap, HashSet},
    error::Error,
    io::Write,
    thread,
    time::Duration,
};

use colored::Colorize;
use itertools::Itertools;
use proconio::input_interactive;
use rand::{seq::SliceRandom, thread_rng};

const DEVELOPER: bool = true;

fn main() {
    let mut game = GameBuilder::new().set_players().set_roles().make();

    let winner = loop {
        game.show_status(Some("The day start".red()));
        if !DEVELOPER {
            thread::sleep(Duration::from_secs(120));
        }
        game.show_status(Some("Voting start".red()));
        wait_enter();
        game.vote();
        if let Some(role) = game.winner() {
            break role;
        }
        wait_enter();
        println!("{}\n", "The night start".red());
        game.night_play();
        if let Some(role) = game.winner() {
            break role;
        }
        wait_enter();
    };

    println!("{} {}", "The winner is".blue(), winner.to_string().red());
}

fn clear() {
    print!("{}[2J", 27 as char);
}

fn wait_enter() {
    std::io::stdin().read_line(&mut String::new()).unwrap();
}

struct GameBuilder {
    players: Vec<String>,
    roles: Vec<(Role, usize)>,
}

impl GameBuilder {
    fn new() -> Self {
        Self {
            players: Vec::new(),
            roles: Vec::new(),
        }
    }

    fn show_status(&self, msg: impl std::fmt::Display) {
        clear();

        println!("{}", msg.to_string().purple());
        println!(
            "{}: {}",
            "Players".blue(),
            self.players
                .iter()
                .map(|v| v.red())
                .join(&", ".blue().to_string())
        );
        println!("{}:", "Roles".blue());
        for (r, n) in self.roles.iter() {
            println!("  {}: {}", r.to_string().blue(), n.to_string().blue());
        }
    }

    fn set_players(mut self) -> Self {
        self.show_status("Input the number of players");
        input_interactive! {
            n: usize,
        }

        assert!(n > 2, "The number of people must be greater than 2");

        self.show_status("Input their names");
        input_interactive! {
            players: [String;n],
        }

        self.players = players;

        self
    }

    fn set_roles(mut self) -> Self {
        self.show_status("Input the number of these roles");

        for role in Role::variants() {
            print!("  {}: ", role.to_string().red());
            std::io::stdout().flush().unwrap();
            input_interactive! {
                num: usize,
            }

            self.roles.push((role, num));
        }

        assert!(
            self.roles
                .iter()
                .find(|(r, _)| r == &Role::Werewolf)
                .is_some_and(|(_, v)| v > &0),
            "The number of Werewolf must be greater than 0"
        );

        self
    }

    fn make(self) -> GameStatus {
        GameStatus::new(self.players, self.roles)
    }
}

struct GameStatus {
    players: HashMap<String, Player>,
    deads: HashMap<String, Player>,
}

impl GameStatus {
    fn new(players: Vec<String>, roles: Vec<(Role, usize)>) -> Self {
        let mut role_list: Vec<Role> = roles
            .into_iter()
            .flat_map(|(role, num)| vec![role; num])
            .collect();
        assert!(
            players.len() == role_list.len(),
            "{}",
            "The number of players differs from the number of roles".red()
        );
        let mut rng = thread_rng();
        role_list.shuffle(&mut rng);

        let game_status = Self {
            players: players
                .into_iter()
                .zip(role_list.into_iter().map(Player::new))
                .collect(),
            deads: HashMap::new(),
        };

        game_status.start();

        game_status
    }

    fn start(&self) {
        for (name, player) in &self.players {
            self.display_clear(Some(format!(
                "You are {} right?\nThen press enter",
                name.red()
            )));
            wait_enter();
            println!(
                "{}",
                format!("Your role is {}", player.role.to_string().red()).blue()
            );
            if player.role == Role::FortuneTeller {
                let mut rng = thread_rng();

                let keys: Vec<&String> = self
                    .players
                    .iter()
                    .filter(|(k, v)| v.role.kind() != RoleKind::Werewolf && k != &name)
                    .map(|(k, _)| k)
                    .collect();

                let tell_name = *keys.choose(&mut rng).unwrap();

                let tell_role = self.players.get(tell_name).unwrap().role;

                println!(
                    "{}",
                    format!(
                        "{}'s role is {}",
                        tell_name.red(),
                        tell_role.kind().to_string().red()
                    )
                    .blue()
                )
            }
            wait_enter();
        }
    }

    fn winner(&self) -> Option<RoleKind> {
        let (werewolfs, villegers): (Vec<Player>, Vec<Player>) = self
            .players
            .values()
            .partition(|v| matches!(v.role, Role::Werewolf));

        if werewolfs.is_empty() {
            Some(RoleKind::Villager)
        } else if werewolfs.len() >= villegers.len() {
            Some(RoleKind::Werewolf)
        } else {
            None
        }
    }

    fn night_play(&mut self) {
        let mut kill_list = HashSet::new();

        for (name, player) in self.players.clone().iter() {
            self.show_status(Some(format!(
                "You are {} right?\nThen press enter",
                name.red()
            )));
            wait_enter();

            player.night_play(self, &mut kill_list);
        }

        let mut killeds = Vec::new();

        for i in &kill_list {
            if let Some(player) = self.players.get_mut(i) {
                if player.killed() {
                    let killed = self.players.remove(i).unwrap();
                    self.deads.insert(i.clone(), killed);
                    killeds.push(i.clone());
                }
            }
        }

        if killeds.is_empty() {
            self.display_clear(Some("Nobody was killed".blue()));
        } else {
            self.display_clear(Some("The killed people are these:".blue()));
            println!(
                "{}\n",
                killeds
                    .iter()
                    .map(|v| v.red())
                    .join(&", ".blue().to_string())
            );
        }
    }

    fn vote(&mut self) {
        let mut vote_counters: HashMap<&String, usize> =
            self.players.keys().map(|k| (k, 0usize)).collect();

        for player in self.players.keys() {
            self.show_status(Some(format!(
                "You are {} right?\nThen input the name of peple you will vote",
                player.red()
            )));
            input_interactive! {
                mut name: String,
            }

            while !self.players.contains_key(&name) {
                println!("{}", "The person is dead or not exist".red());
                println!("{}", "Try again".red());
                input_interactive! {
                    new_name: String,
                }
                name = new_name;
            }

            self.display_clear(None::<String>);

            *vote_counters.get_mut(&name).unwrap() += 1;
        }

        let Some((max_voted_player, _)) = vote_counters.into_iter().only_max_by_key(|(_, p)| (*p))
        else {
            return;
        };

        let max_voted_player = max_voted_player.clone();

        println!(
            "{}\n",
            format!("{} was killed", max_voted_player.red()).blue()
        );

        let v = self.players.remove(&max_voted_player).unwrap();
        self.deads.insert(max_voted_player, v);
    }

    fn show_status(&self, msg: Option<impl std::fmt::Display>) {
        self.display_clear(msg);

        println!(
            "{}: {}",
            "The Alives".blue(),
            self.players
                .keys()
                .map(|v| v.red())
                .join(&", ".blue().to_string())
        );

        println!(
            "{}: {}",
            "The Deads".blue(),
            self.deads
                .keys()
                .map(|s| s.red())
                .join(&", ".blue().to_string())
        )
    }

    fn display_clear(&self, msg: Option<impl std::fmt::Display>) {
        clear();
        if let Some(msg) = msg {
            println!("{}", msg.to_string().purple());
        }
    }
}

trait OnlyMax: Iterator {
    fn only_max_by(
        &mut self,
        f: impl FnMut(&Self::Item, &Self::Item) -> std::cmp::Ordering,
    ) -> Option<Self::Item>;

    fn only_max_by_key<B: Ord>(
        &mut self,
        mut f: impl FnMut(&Self::Item) -> B,
    ) -> Option<Self::Item> {
        self.only_max_by(|a, b| f(a).cmp(&f(b)))
    }

    #[allow(unused)]
    fn only_max(&mut self) -> Option<Self::Item>
    where
        Self::Item: Ord,
    {
        self.only_max_by(|a, b| a.cmp(b))
    }
}

impl<T: Iterator> OnlyMax for T {
    fn only_max_by(
        &mut self,
        mut f: impl FnMut(&Self::Item, &Self::Item) -> std::cmp::Ordering,
    ) -> Option<Self::Item> {
        use std::cmp::Ordering;

        let mut max = self.next()?;

        let mut is_only = true;

        for i in self {
            match f(&i, &max) {
                Ordering::Greater => {
                    max = i;
                    is_only = true
                }
                Ordering::Less => {}
                Ordering::Equal => is_only = false,
            }
        }

        if is_only {
            Some(max)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Player {
    role: Role,
    guarded: bool,
}

impl Player {
    fn new(role: Role) -> Self {
        match role {
            Role::Werewolf => Self {
                role,
                guarded: true,
            },
            _ => Self {
                role,
                guarded: false,
            },
        }
    }

    fn killed(&mut self) -> bool {
        match self.role {
            Role::Werewolf => false,
            _ => {
                if self.guarded {
                    self.guarded = false;
                    false
                } else {
                    true
                }
            }
        }
    }

    fn night_play(&self, status: &mut GameStatus, kill_list: &mut HashSet<String>) {
        match self.role {
            Role::Villager | Role::Maniac => {
                status.show_status(Some("Please input anything and press enter"));
                input_interactive! {
                    _: String,
                }
                println!("{}", "Press enter".blue());
                wait_enter();
            }
            Role::FortuneTeller => {
                status.show_status(Some("Who do you see through?"));
                input_interactive! {
                    mut name: String,
                }

                while !status.players.contains_key(&name) {
                    println!("{}", "The person is dead or not exist".red());
                    println!("{}", "Try again".red());
                    input_interactive! {
                        new_name: String,
                    }
                    name = new_name;
                }

                println!(
                    "{}",
                    format!(
                        "The role is {}",
                        status
                            .players
                            .get(&name)
                            .unwrap()
                            .role
                            .kind()
                            .to_string()
                            .red()
                    )
                    .blue()
                )
            }
            Role::Medium => {
                status.show_status(Some("Who do you see through?"));
                input_interactive! {
                    mut name: String,
                }

                while !status.deads.contains_key(&name) {
                    println!("{}", "The person is alive or not exist".red());
                    println!("{}", "Try again".red());
                    input_interactive! {
                        new_name: String,
                    }
                    name = new_name;
                }

                println!(
                    "{}",
                    format!(
                        "The role is {}",
                        status.deads.get(&name).unwrap().role.to_string().red()
                    )
                    .blue()
                );

                wait_enter();
            }
            Role::Werewolf => {
                status.show_status(Some("Who will you kill?"));
                input_interactive! {
                    mut name: String,
                }

                while !status.players.contains_key(&name) {
                    println!("{}", "The person is dead or not exist".red());
                    println!("{}", "Try again".red());
                    input_interactive! {
                        new_name: String,
                    }
                    name = new_name;
                }

                kill_list.insert(name);

                println!("{}", "Press Enter".blue());

                wait_enter();
            }
            Role::Hunter => {
                status.show_status(Some("Who will you defence?"));
                input_interactive! {
                    mut name: String,
                }

                while !status.players.contains_key(&name) {
                    println!("{}", "The person is dead or not exist".red());
                    println!("{}", "Try again".red());
                    input_interactive! {
                        new_name: String,
                    }
                    name = new_name;
                }

                status.players.get_mut(&name).unwrap().guarded = true;

                println!("{}", "Press Enter".blue());

                wait_enter();
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct MarkerStrErr;

impl std::fmt::Display for MarkerStrErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("No matches")
    }
}

impl Error for MarkerStrErr {}

macro_rules! marker_enum {
    ($vis:vis enum $type:ident {$($variant:ident),+ $(,)?}) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        $vis enum $type {$(
            $variant
        ),+}

        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let s = match self {
                    $(Self::$variant => stringify!($variant)),+
                };

                f.write_str(s)
            }
        }

        impl std::str::FromStr for $type {
            type Err = MarkerStrErr;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $(
                        stringify!($variant) => Ok(Self::$variant),
                    )+
                    _ => Err(MarkerStrErr),
                }
            }
        }

        impl $type {
            #[allow(unused)]
            fn variants() -> Vec<Self> {
                vec![$(Self::$variant),+]
            }
        }
    };
}

marker_enum! {
    enum RoleKind {
        Villager,
        Werewolf,
    }
}

marker_enum! {
    enum Role {
        Villager,
        FortuneTeller,
        Medium,
        Hunter,
        Maniac,
        Werewolf,
    }
}

impl Role {
    fn kind(&self) -> RoleKind {
        match self {
            Self::Werewolf => RoleKind::Werewolf,
            _ => RoleKind::Villager,
        }
    }
}
