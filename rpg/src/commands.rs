//! Analyse des commandes saisies par le joueur.

#[derive(Debug, Clone)]
pub enum Command {
    Help,
    Quit,
    Look,
    Status,
    Inventory,
    Go(String),
    Take(String),
    Use(String),
    Unequip(String),
    Talk(String),
    Shop,
    Buy(String),
    Sell(String),
    Attack(String),
    Cast(String, Option<String>),
    Learn(String),
    Spells,
    Rest,
    Save,
    Load,
    Quests,
    Accept,
    Score,
    Recipes,
    Craft(String),
    Unknown,
}

pub fn parse(input: &str) -> Command {
    let lower = input.trim().to_lowercase();
    if lower.is_empty() {
        return Command::Unknown;
    }
    let mut parts = lower.splitn(2, char::is_whitespace);
    let verb = parts.next().unwrap_or("");
    let arg = parts.next().unwrap_or("").trim().to_string();

    match verb {
        "help" | "h" | "?" => Command::Help,
        "quit" | "exit" | "q" => Command::Quit,
        "look" | "l" => Command::Look,
        "status" | "stats" => Command::Status,
        "inventory" | "inv" | "i" => Command::Inventory,
        "shop" => Command::Shop,
        "rest" | "sleep" => Command::Rest,
        "go" | "move" => Command::Go(arg),
        "north" | "n" => Command::Go("north".into()),
        "south" | "s" => Command::Go("south".into()),
        "east" | "e" => Command::Go("east".into()),
        "west" | "w" => Command::Go("west".into()),
        "take" | "get" | "pickup" => Command::Take(arg),
        "use" | "equip" => Command::Use(arg),
        "unequip" | "deséquiper" | "desequiper" => Command::Unequip(arg),
        "talk" | "speak" => Command::Talk(arg),
        "buy" => Command::Buy(arg),
        "sell" | "vendre" => Command::Sell(arg),
        "attack" | "fight" | "kill" => Command::Attack(arg),
        "cast" => {
            // syntaxe : cast <sort> [cible]
            let mut parts = arg.splitn(2, char::is_whitespace);
            let spell = parts.next().unwrap_or("").to_string();
            let target = parts.next().map(|s| s.trim().to_string());
            Command::Cast(spell, target)
        }
        "learn" => Command::Learn(arg),
        "spells" => Command::Spells,
        "save" => Command::Save,
        "load" => Command::Load,
        "quests" | "journal" => Command::Quests,
        "accept" => Command::Accept,
        "score" | "stats-run" | "résumé" | "resume" => Command::Score,
        "recipes" | "recettes" => Command::Recipes,
        "craft" | "fabriquer" => Command::Craft(arg),
        _ => Command::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_go(input: &str, dir: &str) {
        match parse(input) {
            Command::Go(d) => assert_eq!(d, dir),
            other => panic!("attendu Go({}) reçu {:?}", dir, other),
        }
    }

    #[test]
    fn parse_directions() {
        assert_go("north", "north");
        assert_go("n", "north");
        assert_go("south", "south");
        assert_go("go east", "east");
        assert_go("  GO West  ", "west");
    }

    #[test]
    fn parse_simple_verbs() {
        assert!(matches!(parse("help"), Command::Help));
        assert!(matches!(parse("q"), Command::Quit));
        assert!(matches!(parse("inv"), Command::Inventory));
        assert!(matches!(parse("shop"), Command::Shop));
        assert!(matches!(parse(""), Command::Unknown));
        assert!(matches!(parse("foobar"), Command::Unknown));
    }

    #[test]
    fn parse_with_args() {
        match parse("take Potion de soin") {
            Command::Take(s) => assert_eq!(s, "potion de soin"),
            _ => panic!("Take attendu"),
        }
        match parse("attack gobelin") {
            Command::Attack(s) => assert_eq!(s, "gobelin"),
            _ => panic!("Attack attendu"),
        }
    }

    #[test]
    fn parse_cast_with_target() {
        match parse("cast boule_de_feu gobelin") {
            Command::Cast(s, Some(t)) => {
                assert_eq!(s, "boule_de_feu");
                assert_eq!(t, "gobelin");
            }
            _ => panic!("Cast avec cible attendu"),
        }
        match parse("cast soin") {
            Command::Cast(s, None) => assert_eq!(s, "soin"),
            _ => panic!("Cast sans cible attendu"),
        }
    }
}
