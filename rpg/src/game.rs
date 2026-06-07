//! Boucle de jeu : monde + joueur + cycle jour/nuit (simulation autonome).

use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::combat::fight;
use crate::commands::{parse, Command};
use crate::entity::effects_summary;
use crate::magic::{cast_heal, cast_offensive, find_spell, roll_crit, spell_damage, SPELLS};
use crate::player::{rep_label, Class, Player};
use crate::quest::{Objective, QuestProgress};
use crate::world::{Room, World};

/// Phase du cycle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TimeOfDay {
    Day,
    Night,
}

/// Etat persistant sur disque.
#[derive(Debug, Serialize, Deserialize)]
struct SaveState {
    player: Player,
    rooms: HashMap<u32, Room>,
    turn: u32,
    time: TimeOfDay,
}

const SAVE_PATH: &str = "save.json";

pub struct Game {
    pub world: World,
    pub player: Player,
    pub turn: u32,
    pub time: TimeOfDay,
}

impl Game {
    pub fn new() -> Result<Self, String> {
        let world = World::load()?;
        // Propose de charger une sauvegarde existante.
        if std::path::Path::new(SAVE_PATH).exists() {
            print!("Une sauvegarde existe. La charger ? (o/n) ");
            io::stdout().flush().ok();
            let mut answer = String::new();
            io::stdin()
                .read_line(&mut answer)
                .map_err(|e| e.to_string())?;
            if matches!(
                answer.trim().to_lowercase().as_str(),
                "o" | "oui" | "y" | "yes"
            ) {
                match Self::load_from_disk(world) {
                    Ok(g) => {
                        println!("Sauvegarde chargée.");
                        return Ok(g);
                    }
                    Err(e) => {
                        eprintln!("Échec du chargement ({}). Nouvelle partie.", e);
                        return Self::new_with_world(World::load()?);
                    }
                }
            }
            return Self::new_with_world(world);
        }
        Self::new_with_world(world)
    }

    fn new_with_world(world: World) -> Result<Self, String> {
        print!("Quel est ton nom, aventurier ? ");
        io::stdout().flush().ok();
        let mut name = String::new();
        io::stdin()
            .read_line(&mut name)
            .map_err(|e| e.to_string())?;
        let name = name.trim().to_string();
        let name = if name.is_empty() {
            "Héros".to_string()
        } else {
            name
        };

        // Sélection de classe.
        println!("Choisis ta classe :");
        println!("  1. Guerrier  (Force 14, PV 130, peu de mana)");
        println!("  2. Mage      (Int 12, mana 60, sorts boule_de_feu + soin)");
        println!("  3. Voleur    (équilibré, plus d'or de départ)");
        let class = loop {
            print!("Ton choix [1/2/3] ? ");
            io::stdout().flush().ok();
            let mut buf = String::new();
            io::stdin().read_line(&mut buf).map_err(|e| e.to_string())?;
            match Class::from_input(&buf) {
                Some(c) => break c,
                None => println!("Choix invalide."),
            }
        };
        println!("Tu es maintenant {} le {} !", name, class.label());

        let mut player = Player::new_with_class(name, class);
        player.position = world.start_room;
        Ok(Game {
            world,
            player,
            turn: 0,
            time: TimeOfDay::Day,
        })
    }

    fn load_from_disk(mut world: World) -> Result<Self, String> {
        let raw = fs::read_to_string(SAVE_PATH).map_err(|e| e.to_string())?;
        let state: SaveState = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        world.rooms = state.rooms;
        Ok(Game {
            world,
            player: state.player,
            turn: state.turn,
            time: state.time,
        })
    }

    fn save_to_disk(&self) -> Result<(), String> {
        let state = SaveState {
            player: self.player.clone(),
            rooms: self.world.rooms.clone(),
            turn: self.turn,
            time: self.time,
        };
        let raw = serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?;
        fs::write(SAVE_PATH, raw).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn run(&mut self) {
        println!(
            "\nBienvenue {} ! Tape 'help' pour voir les commandes.\n",
            self.player.name
        );
        self.describe_room();

        loop {
            if !self.player.stats.is_alive() {
                break;
            }

            print!("\n> ");
            io::stdout().flush().ok();
            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() {
                break;
            }
            let cmd = parse(input.trim());
            let consume_turn = self.handle(cmd);
            if consume_turn {
                self.tick();
            }
        }
        println!("Fin de la partie.");
    }

    /// Avance le temps : 1 tour = ~1 unité ; jour/nuit alternent toutes les 5.
    fn tick(&mut self) {
        self.turn += 1;
        let new_time = if (self.turn / 5).is_multiple_of(2) {
            TimeOfDay::Day
        } else {
            TimeOfDay::Night
        };
        if new_time != self.time {
            self.time = new_time;
            match self.time {
                TimeOfDay::Day => println!("(* Le soleil se lève. *)"),
                TimeOfDay::Night => {
                    println!("(* La nuit tombe. Les monstres rôdent... *)");
                    // Simulation : la nuit, les monstres respawn dans la forêt et la caverne.
                    self.respawn_night();
                }
            }
        }
    }

    fn respawn_night(&mut self) {
        // Réinjecte un monstre dans chaque salle clé si elle est vide.
        let spawns = [
            (1u32, "gobelin"),
            (4, "troll"),
            (6, "chauve_souris"),
            (7, "squelette"),
        ];
        for (room_id, monster_id) in spawns {
            let tpl = match self.world.monsters.get(monster_id).cloned() {
                Some(m) => m,
                None => continue,
            };
            if let Some(room) = self.world.room_mut(room_id) {
                if room.monsters.is_empty() {
                    room.monsters
                        .push(crate::entity::MonsterInstance::from_template(&tpl));
                }
            }
        }
    }

    fn describe_room(&self) {
        let room = match self.world.room(self.player.position) {
            Some(r) => r,
            None => return,
        };
        println!("\n--- {} ---", room.name);
        println!("{}", room.description);
        if !room.items.is_empty() {
            let names: Vec<String> = room
                .items
                .iter()
                .filter_map(|id| self.world.items.get(id).map(|i| i.name.clone()))
                .collect();
            println!("Objets visibles : {}", names.join(", "));
        }
        if !room.npcs.is_empty() {
            let names: Vec<String> = room
                .npcs
                .iter()
                .filter_map(|id| self.world.npcs.get(id).map(|n| n.name.clone()))
                .collect();
            println!("Personnages : {}", names.join(", "));
        }
        if !room.monsters.is_empty() {
            let names: Vec<String> = room
                .monsters
                .iter()
                .filter_map(|inst| {
                    self.world.monsters.get(&inst.id).map(|m| {
                        format!(
                            "{} ({} PV){}",
                            m.name,
                            inst.hp,
                            effects_summary(&inst.effects)
                        )
                    })
                })
                .collect();
            println!("MENACES : {}", names.join(", "));
        }
        let exits: Vec<&String> = room.exits.keys().collect();
        let exits_str: Vec<&str> = exits.iter().map(|s| s.as_str()).collect();
        println!("Sorties : {}", exits_str.join(", "));
    }

    /// Retourne true si la commande consomme un tour (le temps avance).
    fn handle(&mut self, cmd: Command) -> bool {
        match cmd {
            Command::Help => {
                self.show_help();
                false
            }
            Command::Quit => {
                self.show_score();
                println!("Au revoir, {} !", self.player.name);
                std::process::exit(0);
            }
            Command::Look => {
                self.describe_room();
                false
            }
            Command::Status => {
                self.player.show_status();
                println!(
                    "Tour {} - {}",
                    self.turn,
                    if self.time == TimeOfDay::Day {
                        "Jour"
                    } else {
                        "Nuit"
                    }
                );
                false
            }
            Command::Inventory => {
                self.player.show_inventory();
                false
            }
            Command::Go(dir) => self.go(&dir),
            Command::Take(name) => self.take(&name),
            Command::Use(name) => {
                self.player.use_item(&name);
                true
            }
            Command::Unequip(slot) => {
                self.player.unequip(&slot);
                false
            }
            Command::Talk(name) => self.talk(&name),
            Command::Shop => self.shop_list(),
            Command::Buy(name) => self.buy(&name),
            Command::Sell(name) => self.sell(&name),
            Command::Attack(name) => self.attack(&name),
            Command::Cast(spell, target) => self.cast(&spell, target.as_deref()),
            Command::Learn(name) => self.learn(&name),
            Command::Spells => {
                self.list_spells();
                false
            }
            Command::Rest => self.rest(),
            Command::Save => {
                match self.save_to_disk() {
                    Ok(()) => println!("Partie sauvegardée dans 'save.json'."),
                    Err(e) => println!("Échec de la sauvegarde : {}", e),
                }
                false
            }
            Command::Quests => {
                self.show_quests();
                false
            }
            Command::Accept => self.accept_quest(),
            Command::Score => {
                self.show_score();
                false
            }
            Command::Recipes => {
                self.show_recipes();
                false
            }
            Command::Craft(name) => self.craft(&name),
            Command::Load => {
                match World::load().and_then(Self::load_from_disk) {
                    Ok(g) => {
                        *self = g;
                        println!("Sauvegarde rechargée.");
                        self.describe_room();
                    }
                    Err(e) => println!("Échec du chargement : {}", e),
                }
                false
            }
            Command::Unknown => {
                println!("Commande inconnue. Tape 'help'.");
                false
            }
        }
    }

    fn show_score(&self) {
        let time_str = if self.time == TimeOfDay::Day {
            "Jour"
        } else {
            "Nuit"
        };
        println!("======== Résumé de partie ========");
        println!(
            "Aventurier         : {} le {}",
            self.player.name,
            self.player.class.label()
        );
        println!("Niveau atteint     : {}", self.player.stats.level);
        println!("Tours joués        : {} ({})", self.turn, time_str);
        println!("Monstres terrassés : {}", self.player.monsters_killed);
        println!("Sorts lancés       : {}", self.player.spells_cast);
        println!("Or amassé          : {}", self.player.gold);
        println!(
            "Réputation         : {} ({})",
            self.player.reputation,
            rep_label(self.player.reputation)
        );
        println!(
            "Quêtes terminées   : {}",
            self.player.quests.iter().filter(|q| q.done).count()
        );
        println!("==================================");
    }

    fn show_help(&self) {
        println!(
            "Commandes :\n\
             - go <direction>      (north/south/east/west)\n\
             - look                décrit la salle\n\
             - status              tes caractéristiques\n\
             - inventory           ton inventaire\n\
             - take <objet>        ramasser un objet\n\
             - use <objet>         utiliser/équiper un objet\n\
             - equip <objet>       alias de 'use' pour équiper\n\
             - unequip <weapon|armor>  déséquiper une arme ou armure\n\
             - talk <pnj>          parler à un PNJ\n\
             - shop                voir les marchandises du marchand présent\n\
             - buy <objet>         acheter un objet\n\
             - sell <objet>        vendre un objet (50% de sa valeur)\n\
             - attack <monstre>    attaquer un monstre\n\
             - cast <sort> [cible] lancer un sort\n\
             - spells              lister tous les sorts existants\n\
             - learn <sort>        apprendre un sort (au sorcier)\n\
             - rest                dormir à l'auberge (5 or)\n\
             - quests              voir tes quêtes en cours\n\
             - accept              accepter la quête du PNJ présent\n\
             - recipes             lister les recettes de craft\n\
             - craft <recette>     fabriquer un objet\n\
             - score               afficher le résumé de partie\n\
             - save                sauvegarder la partie\n\
             - load                recharger la sauvegarde\n\
             - quit                quitter"
        );
    }

    fn go(&mut self, dir: &str) -> bool {
        // Bloque si monstres présents.
        let has_monster = self
            .world
            .room(self.player.position)
            .map(|r| !r.monsters.is_empty())
            .unwrap_or(false);
        if has_monster {
            println!("Tu ne peux pas fuir, un monstre te bloque !");
            return false;
        }
        let next = self
            .world
            .room(self.player.position)
            .and_then(|r| r.exits.get(dir).copied());
        match next {
            Some(id) => {
                self.player.position = id;
                self.describe_room();
                true
            }
            None => {
                println!("Tu ne peux pas aller par là.");
                false
            }
        }
    }

    fn take(&mut self, name: &str) -> bool {
        let pos = self.player.position;
        let lname = name.to_lowercase();
        // Calcule l'index sans conflit d'emprunt.
        let idx = {
            let room = match self.world.room(pos) {
                Some(r) => r,
                None => return false,
            };
            let by_id = room
                .items
                .iter()
                .position(|id| id.to_lowercase().starts_with(&lname));
            by_id.or_else(|| {
                room.items.iter().position(|id| {
                    self.world
                        .items
                        .get(id)
                        .map(|it| it.name.to_lowercase().starts_with(&lname))
                        .unwrap_or(false)
                })
            })
        };
        match idx {
            Some(i) => {
                let id = match self.world.room_mut(pos) {
                    Some(r) => r.items.remove(i),
                    None => return false,
                };
                if let Some(item) = self.world.items.get(&id).cloned() {
                    if item.kind == "Treasure" {
                        println!("Tu ouvres {} et trouves {} or !", item.name, item.value);
                        self.player.gold += item.value;
                    } else {
                        println!("Tu ramasses {}.", item.name);
                        self.player.inventory.push(item);
                        self.on_pickup(&id);
                    }
                }
                true
            }
            None => {
                println!("Pas d'objet '{}' ici.", name);
                false
            }
        }
    }

    fn talk(&self, name: &str) -> bool {
        let room = match self.world.room(self.player.position) {
            Some(r) => r,
            None => return false,
        };
        let lname = name.to_lowercase();
        let npc = room
            .npcs
            .iter()
            .filter_map(|id| self.world.npcs.get(id))
            .find(|n| n.name.to_lowercase().starts_with(&lname) || n.id == lname);
        match npc {
            Some(n) => {
                println!("{} : « {} »", n.name, n.dialogue);
                self.print_npc_reaction(&n.name);
                false
            }
            None => {
                println!("Personne du nom de '{}' ici.", name);
                false
            }
        }
    }

    /// Réaction additionnelle d'un PNJ selon la réputation du joueur.
    fn print_npc_reaction(&self, npc_name: &str) {
        let rep = self.player.reputation;
        let line = match rep {
            i32::MIN..=-1 => format!("{} te jette un regard méfiant.", npc_name),
            0..=9 => return,
            10..=29 => format!("{} a entendu parler de tes exploits.", npc_name),
            30..=59 => format!("{} s'incline : « Le héros du royaume ! »", npc_name),
            _ => format!("{} reste béat devant la légende vivante.", npc_name),
        };
        println!("  ({})", line);
    }

    fn shop_list(&self) -> bool {
        let room = match self.world.room(self.player.position) {
            Some(r) => r,
            None => return false,
        };
        let merchant = room
            .npcs
            .iter()
            .filter_map(|id| self.world.npcs.get(id))
            .find(|n| !n.shop.is_empty());
        match merchant {
            Some(m) => {
                println!("Marchandises de {} :", m.name);
                for id in &m.shop {
                    if let Some(item) = self.world.items.get(id) {
                        println!(
                            "  - {} ({} or) : {}",
                            item.name, item.value, item.description
                        );
                    }
                }
                false
            }
            None => {
                println!("Aucun marchand ici.");
                false
            }
        }
    }

    fn buy(&mut self, name: &str) -> bool {
        let room = match self.world.room(self.player.position) {
            Some(r) => r,
            None => return false,
        };
        let lname = name.to_lowercase();
        let merchant = room
            .npcs
            .iter()
            .filter_map(|id| self.world.npcs.get(id))
            .find(|n| !n.shop.is_empty())
            .cloned();
        let merchant = match merchant {
            Some(m) => m,
            None => {
                println!("Aucun marchand ici.");
                return false;
            }
        };
        let item = merchant.shop.iter().find_map(|id| {
            self.world
                .items
                .get(id)
                .filter(|it| it.name.to_lowercase().starts_with(&lname) || it.id == lname)
        });
        match item.cloned() {
            Some(it) => {
                // Remise marchande pour les héros (rep ≥ 30) : -10%.
                let price = if self.player.reputation >= 30 {
                    (it.value * 9) / 10
                } else {
                    it.value
                };
                if self.player.gold < price {
                    println!("Pas assez d'or ({} requis).", price);
                    false
                } else {
                    self.player.gold -= price;
                    if price < it.value {
                        println!(
                            "Tu achètes {} pour {} or (remise héros : -{} or).",
                            it.name,
                            price,
                            it.value - price
                        );
                    } else {
                        println!("Tu achètes {} pour {} or.", it.name, it.value);
                    }
                    let id = it.id.clone();
                    self.player.inventory.push(it);
                    self.on_pickup(&id);
                    true
                }
            }
            None => {
                println!("Le marchand ne vend pas '{}'.", name);
                false
            }
        }
    }

    fn sell(&mut self, name: &str) -> bool {
        if name.is_empty() {
            println!("Précise l'objet : 'sell <nom>'.");
            return false;
        }
        let room = match self.world.room(self.player.position) {
            Some(r) => r,
            None => return false,
        };
        let merchant_name = room
            .npcs
            .iter()
            .filter_map(|id| self.world.npcs.get(id))
            .find(|n| !n.shop.is_empty())
            .map(|n| n.name.clone());
        let merchant_name = match merchant_name {
            Some(n) => n,
            None => {
                println!("Aucun marchand ici pour racheter tes objets.");
                return false;
            }
        };
        let needle = name.to_lowercase();
        let idx = self.player.inventory.iter().position(|it| {
            it.name.to_lowercase().starts_with(&needle) || it.id.to_lowercase().starts_with(&needle)
        });
        let i = match idx {
            Some(i) => i,
            None => {
                println!("Tu n'as pas '{}' dans ton inventaire.", name);
                return false;
            }
        };
        let price = self.player.inventory[i].sell_price();
        if price == 0 {
            println!(
                "{} ne vaut rien aux yeux du marchand.",
                self.player.inventory[i].name
            );
            return false;
        }
        let item = self.player.inventory.remove(i);
        self.player.gold += price;
        println!(
            "{} t'achète {} pour {} or. (or : {})",
            merchant_name, item.name, price, self.player.gold
        );
        true
    }

    fn attack(&mut self, name: &str) -> bool {
        let pos = self.player.position;
        let lname = name.to_lowercase();
        // Trouve l'index du monstre dans la salle.
        let monster_idx = {
            let room = match self.world.room(pos) {
                Some(r) => r,
                None => return false,
            };
            room.monsters.iter().position(|inst| {
                self.world
                    .monsters
                    .get(&inst.id)
                    .map(|m| m.name.to_lowercase().starts_with(&lname) || m.id == lname)
                    .unwrap_or(false)
            })
        };
        let idx = match monster_idx {
            Some(i) => i,
            None => {
                println!("Pas de monstre '{}' ici.", name);
                return false;
            }
        };
        // Récupère l'instance courante (pour ses PV) et le template.
        let mut instance = match self.world.room(pos) {
            Some(r) => r.monsters[idx].clone(),
            None => return false,
        };
        let tpl = match self.world.monsters.get(&instance.id).cloned() {
            Some(m) => m,
            None => return false,
        };
        let won = fight(&mut self.player, &mut instance, &tpl);
        if won {
            if let Some(room) = self.world.room_mut(pos) {
                room.monsters.remove(idx);
            }
            self.on_kill(&tpl.id);
        }
        true
    }

    fn rest(&mut self) -> bool {
        let room = match self.world.room(self.player.position) {
            Some(r) => r,
            None => return false,
        };
        let has_inn = room.npcs.iter().any(|id| id == "aubergiste");
        if !has_inn {
            println!("Tu ne peux te reposer qu'à l'auberge.");
            return false;
        }
        if self.player.gold < 5 {
            println!("Il faut 5 or pour dormir ici.");
            return false;
        }
        self.player.gold -= 5;
        self.player.stats.hp = self.player.stats.max_hp;
        self.player.stats.mana = self.player.stats.max_mana;
        println!("Tu dors profondément. PV et mana au maximum.");
        true
    }

    fn list_spells(&self) {
        println!("Tous les sorts du jeu :");
        for s in SPELLS {
            println!(
                "  - {} (id: {}) | coût {} mana | {}",
                s.name,
                s.id,
                s.mana_cost,
                if s.heal > 0 {
                    format!("soin {}", s.heal)
                } else {
                    format!("dégâts ~int x {}", s.power)
                }
            );
        }
        if !self.player.spells.is_empty() {
            println!("Sorts connus : {}", self.player.spells.join(", "));
        }
    }

    fn learn(&mut self, name: &str) -> bool {
        // On n'apprend qu'auprès d'un PNJ avec dialogue type "sorcier".
        let here_has_wizard = self
            .world
            .room(self.player.position)
            .map(|r| r.npcs.iter().any(|id| id == "sorcier"))
            .unwrap_or(false);
        if !here_has_wizard {
            println!("Aucun sorcier ici pour t'enseigner.");
            return false;
        }
        let spell = match find_spell(name) {
            Some(s) => s,
            None => {
                println!("Ce sort n'existe pas.");
                return false;
            }
        };
        if self.player.spells.iter().any(|s| s == spell.id) {
            println!("Tu connais déjà {}.", spell.name);
            return false;
        }
        let cost: u32 = 30;
        if self.player.gold < cost {
            println!("Apprendre {} coûte {} or.", spell.name, cost);
            return false;
        }
        self.player.gold -= cost;
        self.player.spells.push(spell.id.to_string());
        println!("Tu apprends {} ! (-{} or)", spell.name, cost);
        true
    }

    fn cast(&mut self, spell_name: &str, target: Option<&str>) -> bool {
        let spell = match find_spell(spell_name) {
            Some(s) => s,
            None => {
                println!("Ce sort n'existe pas.");
                return false;
            }
        };
        if !self.player.spells.iter().any(|s| s == spell.id) {
            println!("Tu ne connais pas ce sort.");
            return false;
        }
        if self.player.stats.mana < spell.mana_cost {
            println!(
                "Pas assez de mana ({} requis, tu en as {}).",
                spell.mana_cost, self.player.stats.mana
            );
            return false;
        }
        self.player.stats.mana -= spell.mana_cost;
        self.player.spells_cast += 1;

        // Sort de soin sur soi.
        if spell.heal > 0 {
            cast_heal(&mut self.player, spell);
            return true;
        }

        // Sort de zone : frappe tous les monstres présents.
        if spell.aoe {
            let pos = self.player.position;
            let base = spell_damage(&self.player, spell);
            // Récupère les ids présents.
            let ids: Vec<String> = self
                .world
                .room(pos)
                .map(|r| r.monsters.iter().map(|m| m.id.clone()).collect())
                .unwrap_or_default();
            if ids.is_empty() {
                println!("Tu déchaînes {} dans le vide.", spell.name);
                return true;
            }
            println!(
                "Un orage furieux s'abat ! {} déchaîne sa fureur.",
                spell.name
            );
            // Pré-calcul des noms (le borrow checker n'aime pas lire `world.monsters`
            // pendant qu'on tient une mut ref sur la salle).
            let monsters_catalog: std::collections::HashMap<String, String> = ids
                .iter()
                .filter_map(|id| {
                    self.world
                        .monsters
                        .get(id)
                        .map(|m| (id.clone(), m.name.clone()))
                })
                .collect();
            // Applique les dégâts ; collecte les morts.
            let mut killed_ids: Vec<String> = Vec::new();
            if let Some(room) = self.world.room_mut(pos) {
                for inst in room.monsters.iter_mut() {
                    let (dmg, crit) = roll_crit(base);
                    inst.hp -= dmg;
                    let tag = if crit { " *CRITIQUE !*" } else { "" };
                    let display_name = monsters_catalog
                        .get(&inst.id)
                        .cloned()
                        .unwrap_or_else(|| inst.id.clone());
                    println!("  -> {} subit {} dégats{}.", display_name, dmg, tag);
                }
                room.monsters.retain(|inst| {
                    if inst.hp <= 0 {
                        killed_ids.push(inst.id.clone());
                        false
                    } else {
                        true
                    }
                });
            }
            // Récompenses + hooks pour chaque mort.
            for id in &killed_ids {
                if let Some(tpl) = self.world.monsters.get(id).cloned() {
                    println!(
                        ">> {} est foudroyé ! +{} XP, +{} or.",
                        tpl.name, tpl.xp, tpl.gold
                    );
                    self.player.gold += tpl.gold;
                    self.player.stats.gain_xp(tpl.xp);
                }
                self.on_kill(id);
            }
            // Riposte des survivants.
            let survivors: Vec<String> = self
                .world
                .room(pos)
                .map(|r| r.monsters.iter().map(|m| m.id.clone()).collect())
                .unwrap_or_default();
            for sid in survivors {
                if let Some(tpl) = self.world.monsters.get(&sid).cloned() {
                    let mdmg = (tpl.attack - self.player.defense()).max(1);
                    self.player.stats.hp -= mdmg;
                    println!(
                        "{} riposte pour {} dégats. ({} PV restants)",
                        tpl.name,
                        mdmg,
                        self.player.stats.hp.max(0)
                    );
                }
            }
            if !self.player.stats.is_alive() {
                println!(">> Tu es mort... Game over.");
                return true;
            }
            return true;
        }

        // Sort offensif : il faut une cible.
        let pos = self.player.position;
        let target_name = match target {
            Some(t) if !t.is_empty() => t.to_lowercase(),
            _ => {
                println!("Précise la cible : cast {} <monstre>", spell.id);
                return true;
            }
        };
        let monster_idx = self.world.room(pos).and_then(|r| {
            r.monsters.iter().position(|inst| {
                self.world
                    .monsters
                    .get(&inst.id)
                    .map(|m| m.name.to_lowercase().starts_with(&target_name) || m.id == target_name)
                    .unwrap_or(false)
            })
        });
        let idx = match monster_idx {
            Some(i) => i,
            None => {
                println!("Pas de monstre '{}' ici.", target_name);
                return true;
            }
        };
        let mut instance = match self.world.room(pos) {
            Some(r) => r.monsters[idx].clone(),
            None => return true,
        };
        let tpl = match self.world.monsters.get(&instance.id).cloned() {
            Some(m) => m,
            None => return true,
        };
        let killed = cast_offensive(&mut self.player, &mut instance, &tpl, spell);
        if killed {
            if let Some(room) = self.world.room_mut(pos) {
                room.monsters.remove(idx);
            }
            self.on_kill(&tpl.id);
        } else {
            // Persiste les PV restants de l'instance.
            if let Some(room) = self.world.room_mut(pos) {
                if let Some(slot) = room.monsters.get_mut(idx) {
                    *slot = instance.clone();
                }
            }
            // Le monstre riposte.
            let mdmg = tpl.attack;
            self.player.stats.hp -= mdmg;
            println!(
                "{} te blesse de {} dégâts. ({} PV restants)",
                tpl.name,
                mdmg,
                self.player.stats.hp.max(0)
            );
        }
        true
    }

    // ===== Quêtes =====

    fn show_quests(&self) {
        if self.player.quests.is_empty() {
            println!("Aucune quête en cours.");
            return;
        }
        println!("Journal de quêtes :");
        for qp in &self.player.quests {
            let q = match self.world.quests.get(&qp.id) {
                Some(q) => q,
                None => continue,
            };
            let status = if qp.done {
                "TERMINÉE".to_string()
            } else {
                format!("{}/{}", qp.progress, q.objective.target_count())
            };
            println!(
                "  - [{}] {} : {} ({})",
                status, q.title, q.description, qp.id
            );
        }
    }

    fn accept_quest(&mut self) -> bool {
        let pos = self.player.position;
        // Trouve un PNJ de la salle qui propose une quête.
        let quest_id = {
            let room = match self.world.room(pos) {
                Some(r) => r,
                None => return false,
            };
            room.npcs
                .iter()
                .filter_map(|id| self.world.npcs.get(id))
                .find_map(|n| n.quest.clone())
        };
        let qid = match quest_id {
            Some(q) => q,
            None => {
                println!("Personne n'a de quête à te proposer ici.");
                return false;
            }
        };
        if self.player.quests.iter().any(|qp| qp.id == qid) {
            println!("Tu as déjà cette quête.");
            return false;
        }
        let q = match self.world.quests.get(&qid) {
            Some(q) => q.clone(),
            None => {
                println!("Quête introuvable.");
                return false;
            }
        };
        println!(
            ">> Quête acceptée : {} - {} (récompense : {} or, {} XP)",
            q.title, q.description, q.reward_gold, q.reward_xp
        );
        self.player.quests.push(QuestProgress::new(&qid));
        false
    }

    /// Hook appelé après un kill : incrémente progression Kill.
    fn on_kill(&mut self, monster_id: &str) {
        self.player.monsters_killed += 1;
        // Loot : ajoute les drops du monstre à l'inventaire.
        if let Some(tpl) = self.world.monsters.get(monster_id) {
            let drops = tpl.drops.clone();
            for drop_id in drops {
                if let Some(item) = self.world.items.get(&drop_id).cloned() {
                    println!("  >> Tu récupères {}.", item.name);
                    self.player.inventory.push(item);
                    self.on_pickup(&drop_id);
                }
            }
        }
        if monster_id == "dragon_noir" {
            println!();
            println!("============================================");
            println!("  ★  VICTOIRE !  Le Dragon Noir s'effondre  ★");
            println!("  La paix revient sur les terres du royaume.");
            println!("============================================");
            println!();
        }
        let mut completions: Vec<String> = Vec::new();
        for qp in self.player.quests.iter_mut() {
            if qp.done {
                continue;
            }
            let q = match self.world.quests.get(&qp.id) {
                Some(q) => q,
                None => continue,
            };
            if let Objective::Kill {
                monster_id: target,
                count,
            } = &q.objective
            {
                if target == monster_id {
                    qp.progress += 1;
                    println!("  [Quête {}] {}/{}", q.title, qp.progress, count);
                    if qp.progress >= *count {
                        qp.done = true;
                        completions.push(qp.id.clone());
                    }
                }
            }
        }
        for id in completions {
            self.complete_quest(&id);
        }
    }

    /// Hook appelé après ramassage/achat : recompte les objets de l'inventaire.
    fn on_pickup(&mut self, item_id: &str) {
        let mut completions: Vec<String> = Vec::new();
        for qp in self.player.quests.iter_mut() {
            if qp.done {
                continue;
            }
            let q = match self.world.quests.get(&qp.id) {
                Some(q) => q,
                None => continue,
            };
            if let Objective::Collect {
                item_id: target,
                count,
            } = &q.objective
            {
                if target == item_id {
                    let owned: u32 = self
                        .player
                        .inventory
                        .iter()
                        .filter(|it| &it.id == target)
                        .count() as u32;
                    qp.progress = owned;
                    println!("  [Quête {}] {}/{}", q.title, qp.progress, count);
                    if qp.progress >= *count {
                        qp.done = true;
                        completions.push(qp.id.clone());
                    }
                }
            }
        }
        for id in completions {
            self.complete_quest(&id);
        }
    }

    fn complete_quest(&mut self, quest_id: &str) {
        let q = match self.world.quests.get(quest_id) {
            Some(q) => q.clone(),
            None => return,
        };
        println!(
            ">> Quête terminée : {} ! +{} or, +{} XP, +10 réputation",
            q.title, q.reward_gold, q.reward_xp
        );
        self.player.gold += q.reward_gold;
        self.player.stats.gain_xp(q.reward_xp);
        self.player.reputation += 10;
    }

    /// Liste les recettes de craft connues, avec dispo des ingrédients.
    fn show_recipes(&self) {
        if self.world.recipes.is_empty() {
            println!("Aucune recette connue.");
            return;
        }
        println!("Recettes disponibles :");
        // Tri stable par id pour un affichage déterministe.
        let mut keys: Vec<&String> = self.world.recipes.keys().collect();
        keys.sort();
        for k in keys {
            let r = &self.world.recipes[k];
            let parts: Vec<String> = r
                .inputs
                .iter()
                .map(|ing| {
                    let have = self.player.count_item(&ing.item_id);
                    let name = self
                        .world
                        .items
                        .get(&ing.item_id)
                        .map(|i| i.name.as_str())
                        .unwrap_or(&ing.item_id);
                    let mark = if have >= ing.count { "✓" } else { "✗" };
                    format!("{} x{} ({}/{}) {}", name, ing.count, have, ing.count, mark)
                })
                .collect();
            let out_name = self
                .world
                .items
                .get(&r.output_id)
                .map(|i| i.name.as_str())
                .unwrap_or(&r.output_id);
            println!(
                "  - {} : {} -> {} x{}",
                r.name,
                parts.join(" + "),
                out_name,
                r.output_count.max(1)
            );
        }
        println!("Tape 'craft <nom de recette>' pour fabriquer.");
    }

    /// Tente de fabriquer un objet selon une recette.
    fn craft(&mut self, name: &str) -> bool {
        if name.is_empty() {
            println!("Précise la recette : 'craft <nom>'.");
            return false;
        }
        let recipe = match self.world.find_recipe(name) {
            Some(r) => r.clone(),
            None => {
                println!("Recette inconnue : {}.", name);
                return false;
            }
        };
        match self.player.craft(&recipe, &self.world.items) {
            Ok(()) => {
                let out_name = self
                    .world
                    .items
                    .get(&recipe.output_id)
                    .map(|i| i.name.clone())
                    .unwrap_or_else(|| recipe.output_id.clone());
                println!(
                    ">> Fabrication réussie : {} x{} !",
                    out_name,
                    recipe.output_count.max(1)
                );
                // Met à jour les quêtes de collecte si la sortie correspond.
                self.on_pickup(&recipe.output_id);
                true
            }
            Err(e) => {
                println!("Impossible de fabriquer : {}", e);
                false
            }
        }
    }
}
