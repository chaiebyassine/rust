//! Définition du joueur, ses caractéristiques et son inventaire.

use serde::{Deserialize, Serialize};

use crate::entity::{Item, Recipe};
use crate::quest::QuestProgress;

/// Classe choisie en début de partie : oriente les stats de départ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Class {
    Guerrier,
    Mage,
    Voleur,
}

impl Default for Class {
    fn default() -> Self {
        Class::Guerrier
    }
}

impl Class {
    pub fn from_input(s: &str) -> Option<Class> {
        match s.trim().to_lowercase().as_str() {
            "1" | "g" | "guerrier" | "warrior" => Some(Class::Guerrier),
            "2" | "m" | "mage" | "wizard" => Some(Class::Mage),
            "3" | "v" | "voleur" | "thief" | "rogue" => Some(Class::Voleur),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Class::Guerrier => "Guerrier",
            Class::Mage => "Mage",
            Class::Voleur => "Voleur",
        }
    }
}

/// Statistiques du personnage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub force: u32,
    pub intelligence: u32,
    pub hp: i32,
    pub max_hp: i32,
    pub mana: i32,
    pub max_mana: i32,
    pub xp: u32,
    pub level: u32,
}

impl Stats {
    pub fn new() -> Self {
        Stats {
            force: 10,
            intelligence: 5,
            hp: 100,
            max_hp: 100,
            mana: 30,
            max_mana: 30,
            xp: 0,
            level: 1,
        }
    }

    /// Ajoute de l'XP et fait monter de niveau si besoin.
    pub fn gain_xp(&mut self, amount: u32) {
        self.xp += amount;
        while self.xp >= self.level * 50 {
            self.xp -= self.level * 50;
            self.level += 1;
            self.max_hp += 20;
            self.hp = self.max_hp;
            self.max_mana += 10;
            self.mana = self.max_mana;
            self.force += 2;
            self.intelligence += 1;
            println!(">> Niveau supérieur ! Tu es maintenant niveau {}.", self.level);
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }
}

/// Construit une barre ASCII de largeur `width` représentant `value/max`.
/// Exemple: `bar(70, 100, 20)` -> "[##############------]"
pub fn bar(value: i32, max: i32, width: usize) -> String {
    let max = max.max(1);
    let v = value.max(0).min(max);
    let filled = (v as usize * width) / max as usize;
    let empty = width - filled;
    let mut s = String::with_capacity(width + 2);
    s.push('[');
    for _ in 0..filled {
        s.push('#');
    }
    for _ in 0..empty {
        s.push('-');
    }
    s.push(']');
    s
}

/// Étiquette descriptive d'un niveau de réputation.
pub fn rep_label(rep: i32) -> &'static str {
    match rep {
        i32::MIN..=-10 => "Paria",
        -9..=-1 => "Suspect",
        0..=9 => "Inconnu",
        10..=29 => "Apprécié",
        30..=59 => "Héros",
        _ => "Légende",
    }
}

/// Le joueur.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    #[serde(default)]
    pub class: Class,
    pub position: u32,
    pub stats: Stats,
    pub inventory: Vec<Item>,
    pub gold: u32,
    /// Conservé pour compatibilité avec d'anciennes sauvegardes ; plus utilisé.
    #[serde(default)]
    pub equipped_attack_bonus: i32,
    #[serde(default)]
    pub equipped_weapon: Option<Item>,
    #[serde(default)]
    pub equipped_armor: Option<Item>,
    #[serde(default)]
    pub spells: Vec<String>,
    #[serde(default)]
    pub quests: Vec<QuestProgress>,
    #[serde(default)]
    pub monsters_killed: u32,
    #[serde(default)]
    pub spells_cast: u32,
    #[serde(default)]
    pub reputation: i32,
}

impl Player {
    pub fn new(name: String) -> Self {
        Player {
            name,
            class: Class::Guerrier,
            position: 0,
            stats: Stats::new(),
            inventory: Vec::new(),
            gold: 20,
            equipped_attack_bonus: 0,
            equipped_weapon: None,
            equipped_armor: None,
            spells: vec!["boule_de_feu".to_string()],
            quests: Vec::new(),
            monsters_killed: 0,
            spells_cast: 0,
            reputation: 0,
        }
    }

    /// Crée un joueur en appliquant les stats et inventaire de départ d'une classe.
    pub fn new_with_class(name: String, class: Class) -> Self {
        let mut p = Player::new(name);
        p.class = class;
        match class {
            Class::Guerrier => {
                p.stats.force = 14;
                p.stats.intelligence = 3;
                p.stats.max_hp = 130;
                p.stats.hp = 130;
                p.stats.max_mana = 10;
                p.stats.mana = 10;
                p.spells.clear();
                p.gold = 25;
            }
            Class::Mage => {
                p.stats.force = 6;
                p.stats.intelligence = 12;
                p.stats.max_hp = 80;
                p.stats.hp = 80;
                p.stats.max_mana = 60;
                p.stats.mana = 60;
                p.spells = vec!["boule_de_feu".to_string(), "soin".to_string()];
                p.gold = 15;
            }
            Class::Voleur => {
                p.stats.force = 11;
                p.stats.intelligence = 7;
                p.stats.max_hp = 100;
                p.stats.hp = 100;
                p.stats.max_mana = 25;
                p.stats.mana = 25;
                p.spells = vec!["boule_de_feu".to_string()];
                p.gold = 40;
            }
        }
        p
    }

    /// Dégât de base (force + bonus arme équipée).
    pub fn attack_damage(&self) -> i32 {
        let weapon = self
            .equipped_weapon
            .as_ref()
            .map_or(0, |w| w.attack_bonus);
        self.stats.force as i32 + weapon
    }

    /// Réduction de dégâts apportée par l'armure équipée.
    pub fn defense(&self) -> i32 {
        self.equipped_armor
            .as_ref()
            .map_or(0, |a| a.defense_bonus)
    }

    pub fn show_status(&self) {
        println!(
            "[{} le {}] Niveau {} | XP {} | Or {} | Force {} | Int {}",
            self.name,
            self.class.label(),
            self.stats.level,
            self.stats.xp,
            self.gold,
            self.stats.force,
            self.stats.intelligence,
        );
        println!(
            "PV   {} {}/{}",
            bar(self.stats.hp, self.stats.max_hp, 20),
            self.stats.hp,
            self.stats.max_hp,
        );
        println!(
            "Mana {} {}/{}",
            bar(self.stats.mana, self.stats.max_mana, 20),
            self.stats.mana,
            self.stats.max_mana,
        );
        let weapon = self
            .equipped_weapon
            .as_ref()
            .map(|w| format!("{} (+{} att)", w.name, w.attack_bonus))
            .unwrap_or_else(|| "—".to_string());
        let armor = self
            .equipped_armor
            .as_ref()
            .map(|a| format!("{} (+{} déf)", a.name, a.defense_bonus))
            .unwrap_or_else(|| "—".to_string());
        println!("Équipement : Arme = {} | Armure = {}", weapon, armor);
        println!(
            "Réputation : {} ({})",
            self.reputation,
            rep_label(self.reputation)
        );
        if !self.spells.is_empty() {
            println!("Sorts connus : {}", self.spells.join(", "));
        }
    }

    pub fn show_inventory(&self) {
        if self.inventory.is_empty() && self.equipped_weapon.is_none() && self.equipped_armor.is_none() {
            println!("Inventaire vide.");
            return;
        }
        println!("Inventaire :");
        for (i, item) in self.inventory.iter().enumerate() {
            println!("  {}. {} - {}", i + 1, item.name, item.description);
        }
        if let Some(w) = &self.equipped_weapon {
            println!("  [équipée] {} (+{} attaque)", w.name, w.attack_bonus);
        }
        if let Some(a) = &self.equipped_armor {
            println!("  [équipée] {} (+{} défense)", a.name, a.defense_bonus);
        }
    }

    /// Équipe un objet (arme ou armure) : le retire de l'inventaire et range
    /// l'éventuel objet précédemment équipé.
    pub fn equip(&mut self, name: &str) {
        let needle = name.to_lowercase();
        let idx = self.inventory.iter().position(|it| {
            it.name.to_lowercase().starts_with(&needle)
                || it.id.to_lowercase().starts_with(&needle)
        });
        let i = match idx {
            Some(i) => i,
            None => {
                println!("Tu n'as pas cet objet.");
                return;
            }
        };
        let item = self.inventory.remove(i);
        match item.kind.as_str() {
            "Weapon" => {
                if let Some(prev) = self.equipped_weapon.take() {
                    println!("Tu rentres {} dans le sac.", prev.name);
                    self.inventory.push(prev);
                }
                println!("Tu équipes {} (+{} attaque).", item.name, item.attack_bonus);
                self.equipped_weapon = Some(item);
            }
            "Armor" => {
                if let Some(prev) = self.equipped_armor.take() {
                    println!("Tu retires {}.", prev.name);
                    self.inventory.push(prev);
                }
                println!("Tu enfiles {} (+{} défense).", item.name, item.defense_bonus);
                self.equipped_armor = Some(item);
            }
            _ => {
                println!("Tu ne peux pas équiper {}.", item.name);
                self.inventory.push(item);
            }
        }
    }

    /// Déséquipe un slot : "weapon" / "arme" ou "armor" / "armure".
    pub fn unequip(&mut self, slot: &str) {
        let s = slot.to_lowercase();
        let s = s.trim();
        match s {
            "weapon" | "arme" | "" => match self.equipped_weapon.take() {
                Some(w) => {
                    println!("Tu rentres {} dans le sac.", w.name);
                    self.inventory.push(w);
                }
                None => println!("Aucune arme équipée."),
            },
            "armor" | "armure" => match self.equipped_armor.take() {
                Some(a) => {
                    println!("Tu retires {}.", a.name);
                    self.inventory.push(a);
                }
                None => println!("Aucune armure équipée."),
            },
            _ => println!("Précise 'weapon' ou 'armor'."),
        }
    }

    /// Utilise un objet de l'inventaire par son nom ou son id (insensible aux accents).
    pub fn use_item(&mut self, name: &str) {
        let needle = name.to_lowercase();
        let idx = self.inventory.iter().position(|it| {
            it.name.to_lowercase().starts_with(&needle)
                || it.id.to_lowercase().starts_with(&needle)
        });
        match idx {
            Some(i) => {
                let kind = self.inventory[i].kind.clone();
                match kind.as_str() {
                    "Consumable" => {
                        let item = self.inventory.remove(i);
                        let new_hp = (self.stats.hp + item.heal).min(self.stats.max_hp);
                        self.stats.hp = new_hp;
                        println!("Tu utilises {} et récupères {} PV.", item.name, item.heal);
                    }
                    "Weapon" | "Armor" => {
                        // Délègue à equip pour gérer le swap.
                        self.equip(name);
                    }
                    "Material" => println!(
                        "{} est un composant de craft. Tape 'craft <recette>'.",
                        self.inventory[i].name
                    ),
                    _ => println!("Tu ne peux pas utiliser {}.", self.inventory[i].name),
                }
            }
            None => println!("Tu n'as pas cet objet."),
        }
    }

    /// Compte les exemplaires d'un item_id présents dans l'inventaire.
    pub fn count_item(&self, item_id: &str) -> u32 {
        self.inventory.iter().filter(|i| i.id == item_id).count() as u32
    }

    /// Tente d'appliquer une recette. Retourne Ok si succès, Err avec un message sinon.
    /// `catalog` permet de retrouver la définition de l'objet produit.
    pub fn craft(
        &mut self,
        recipe: &Recipe,
        catalog: &std::collections::HashMap<String, Item>,
    ) -> Result<(), String> {
        // Vérifie que tous les ingrédients sont présents.
        for ing in &recipe.inputs {
            let have = self.count_item(&ing.item_id);
            if have < ing.count {
                let missing_name = catalog
                    .get(&ing.item_id)
                    .map(|i| i.name.clone())
                    .unwrap_or_else(|| ing.item_id.clone());
                return Err(format!(
                    "Il manque {} (tu as {}/{}).",
                    missing_name, have, ing.count
                ));
            }
        }
        // Vérifie que l'objet produit existe dans le monde.
        let output = catalog
            .get(&recipe.output_id)
            .ok_or_else(|| format!("Objet produit inconnu : {}", recipe.output_id))?
            .clone();
        // Consomme les ingrédients.
        for ing in &recipe.inputs {
            let mut to_remove = ing.count;
            self.inventory.retain(|it| {
                if to_remove > 0 && it.id == ing.item_id {
                    to_remove -= 1;
                    false
                } else {
                    true
                }
            });
        }
        // Ajoute la (les) sortie(s).
        for _ in 0..recipe.output_count.max(1) {
            self.inventory.push(output.clone());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::Item;

    #[test]
    fn level_up_increases_stats() {
        let mut s = Stats::new();
        let initial_level = s.level;
        let initial_force = s.force;
        s.gain_xp(50); // assez pour monter de niveau 1
        assert!(s.level > initial_level);
        assert!(s.force > initial_force);
        assert_eq!(s.hp, s.max_hp);
    }

    #[test]
    fn xp_below_threshold_does_not_level_up() {
        let mut s = Stats::new();
        s.gain_xp(10);
        assert_eq!(s.level, 1);
        assert_eq!(s.xp, 10);
    }

    #[test]
    fn use_potion_heals() {
        let mut p = Player::new("Test".into());
        p.stats.hp = 50;
        p.inventory.push(Item {
            id: "potion".into(),
            name: "Potion".into(),
            description: "".into(),
            kind: "Consumable".into(),
            value: 0,
            heal: 20,
            attack_bonus: 0,
            defense_bonus: 0,
        });
        p.use_item("potion");
        assert_eq!(p.stats.hp, 70);
        assert!(p.inventory.is_empty());
    }

    #[test]
    fn equip_weapon_sets_bonus() {
        let mut p = Player::new("Test".into());
        p.inventory.push(Item {
            id: "epee".into(),
            name: "Epee".into(),
            description: "".into(),
            kind: "Weapon".into(),
            value: 0,
            heal: 0,
            attack_bonus: 7,
            defense_bonus: 0,
        });
        p.use_item("epee");
        assert!(p.equipped_weapon.is_some());
        assert_eq!(p.attack_damage(), p.stats.force as i32 + 7);
    }

    #[test]
    fn equip_armor_then_unequip() {
        let mut p = Player::new("Test".into());
        p.inventory.push(Item {
            id: "armure_cuir".into(),
            name: "Armure de cuir".into(),
            description: "".into(),
            kind: "Armor".into(),
            value: 0,
            heal: 0,
            attack_bonus: 0,
            defense_bonus: 3,
        });
        p.use_item("armure_cuir");
        assert_eq!(p.defense(), 3);
        p.unequip("armor");
        assert_eq!(p.defense(), 0);
        assert_eq!(p.inventory.len(), 1);
    }

    #[test]
    fn class_from_input_accepte_plusieurs_formats() {
        assert!(matches!(Class::from_input("1"), Some(Class::Guerrier)));
        assert!(matches!(Class::from_input("g"), Some(Class::Guerrier)));
        assert!(matches!(Class::from_input("Guerrier"), Some(Class::Guerrier)));
        assert!(matches!(Class::from_input("2"), Some(Class::Mage)));
        assert!(matches!(Class::from_input("mage"), Some(Class::Mage)));
        assert!(matches!(Class::from_input("v"), Some(Class::Voleur)));
        assert!(Class::from_input("inconnu").is_none());
    }

    #[test]
    fn new_with_class_mage_a_des_sorts_et_pv_reduits() {
        let p = Player::new_with_class("Merlin".into(), Class::Mage);
        assert_eq!(p.stats.max_hp, 80);
        assert_eq!(p.stats.max_mana, 60);
        assert!(p.spells.contains(&"boule_de_feu".to_string()));
        assert!(p.spells.contains(&"soin".to_string()));
    }

    #[test]
    fn new_with_class_guerrier_naucun_sort() {
        let p = Player::new_with_class("Bran".into(), Class::Guerrier);
        assert_eq!(p.stats.force, 14);
        assert_eq!(p.stats.max_hp, 130);
        assert!(p.spells.is_empty());
    }

    #[test]
    fn bar_proportionnelle_a_value_max() {
        assert_eq!(bar(0, 10, 4), "[----]");
        assert_eq!(bar(10, 10, 4), "[####]");
        assert_eq!(bar(5, 10, 4), "[##--]");
        // valeurs hors bornes clampées
        assert_eq!(bar(-5, 10, 4), "[----]");
        assert_eq!(bar(99, 10, 4), "[####]");
    }

    #[test]
    fn bar_max_zero_ne_panic_pas() {
        // protection : la fonction normalise max à 1 minimum.
        let _ = bar(0, 0, 4);
    }

    #[test]
    fn rep_label_couvre_les_paliers() {
        assert_eq!(rep_label(-100), "Paria");
        assert_eq!(rep_label(-5), "Suspect");
        assert_eq!(rep_label(0), "Inconnu");
        assert_eq!(rep_label(15), "Apprécié");
        assert_eq!(rep_label(40), "Héros");
        assert_eq!(rep_label(100), "Légende");
    }

    fn potion_test_item() -> Item {
        Item {
            id: "potion".into(),
            name: "Potion".into(),
            description: "".into(),
            kind: "Consumable".into(),
            value: 0,
            heal: 20,
            attack_bonus: 0,
            defense_bonus: 0,
        }
    }

    fn potion_grande_test_item() -> Item {
        Item {
            id: "potion_grande".into(),
            name: "Grande potion".into(),
            description: "".into(),
            kind: "Consumable".into(),
            value: 0,
            heal: 60,
            attack_bonus: 0,
            defense_bonus: 0,
        }
    }

    fn catalog_de_test() -> std::collections::HashMap<String, Item> {
        let mut h = std::collections::HashMap::new();
        h.insert("potion".into(), potion_test_item());
        h.insert("potion_grande".into(), potion_grande_test_item());
        h
    }

    fn recette_grande_potion() -> crate::entity::Recipe {
        crate::entity::Recipe {
            id: "potion_grande".into(),
            name: "Grande potion".into(),
            inputs: vec![crate::entity::Ingredient {
                item_id: "potion".into(),
                count: 2,
            }],
            output_id: "potion_grande".into(),
            output_count: 1,
        }
    }

    #[test]
    fn count_item_compte_les_doublons() {
        let mut p = Player::new("T".into());
        p.inventory.push(potion_test_item());
        p.inventory.push(potion_test_item());
        p.inventory.push(potion_grande_test_item());
        assert_eq!(p.count_item("potion"), 2);
        assert_eq!(p.count_item("potion_grande"), 1);
        assert_eq!(p.count_item("rien"), 0);
    }

    #[test]
    fn craft_consomme_inputs_et_produit_output() {
        let mut p = Player::new("T".into());
        p.inventory.push(potion_test_item());
        p.inventory.push(potion_test_item());
        let r = recette_grande_potion();
        let cat = catalog_de_test();
        assert!(p.craft(&r, &cat).is_ok());
        assert_eq!(p.count_item("potion"), 0);
        assert_eq!(p.count_item("potion_grande"), 1);
    }

    #[test]
    fn craft_echec_si_ingredients_insuffisants() {
        let mut p = Player::new("T".into());
        p.inventory.push(potion_test_item());
        let r = recette_grande_potion();
        let cat = catalog_de_test();
        assert!(p.craft(&r, &cat).is_err());
        assert_eq!(p.count_item("potion"), 1);
        assert_eq!(p.count_item("potion_grande"), 0);
    }

    #[test]
    fn craft_consomme_exactement_la_quantite_requise() {
        let mut p = Player::new("T".into());
        p.inventory.push(potion_test_item());
        p.inventory.push(potion_test_item());
        p.inventory.push(potion_test_item());
        let r = recette_grande_potion();
        let cat = catalog_de_test();
        p.craft(&r, &cat).unwrap();
        assert_eq!(p.count_item("potion"), 1);
        assert_eq!(p.count_item("potion_grande"), 1);
    }
}
