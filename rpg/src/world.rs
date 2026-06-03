//! Le monde : chargement JSON, salles, recherches.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::entity::{Item, Monster, MonsterInstance, Npc, Recipe};
use crate::quest::Quest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: u32,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub exits: HashMap<String, u32>,
    #[serde(default)]
    pub items: Vec<String>,
    #[serde(default)]
    pub npcs: Vec<String>,
    #[serde(default)]
    pub monsters: Vec<MonsterInstance>,
}

/// Format JSON brut (les monstres sont des identifiants).
#[derive(Debug, Deserialize)]
struct RoomFile {
    id: u32,
    name: String,
    description: String,
    #[serde(default)]
    exits: HashMap<String, u32>,
    #[serde(default)]
    items: Vec<String>,
    #[serde(default)]
    npcs: Vec<String>,
    #[serde(default)]
    monsters: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WorldFile {
    start_room: u32,
    rooms: Vec<RoomFile>,
    items: Vec<Item>,
    npcs: Vec<Npc>,
    monsters: Vec<Monster>,
    #[serde(default)]
    quests: Vec<Quest>,
    #[serde(default)]
    recipes: Vec<Recipe>,
}

pub struct World {
    pub start_room: u32,
    pub rooms: HashMap<u32, Room>,
    pub items: HashMap<String, Item>,
    pub npcs: HashMap<String, Npc>,
    pub monsters: HashMap<String, Monster>,
    pub quests: HashMap<String, Quest>,
    pub recipes: HashMap<String, Recipe>,
}

impl World {
    pub fn load() -> Result<Self, String> {
        let raw = include_str!("../../data/world.json");
        let wf: WorldFile = serde_json::from_str(raw)
            .map_err(|e| format!("JSON invalide : {}", e))?;
        let monsters: HashMap<String, Monster> =
            wf.monsters.into_iter().map(|m| (m.id.clone(), m)).collect();
        let rooms = wf
            .rooms
            .into_iter()
            .map(|rf| {
                let monsters_in_room: Vec<MonsterInstance> = rf
                    .monsters
                    .iter()
                    .filter_map(|id| monsters.get(id).map(MonsterInstance::from_template))
                    .collect();
                let room = Room {
                    id: rf.id,
                    name: rf.name,
                    description: rf.description,
                    exits: rf.exits,
                    items: rf.items,
                    npcs: rf.npcs,
                    monsters: monsters_in_room,
                };
                (rf.id, room)
            })
            .collect();
        Ok(World {
            start_room: wf.start_room,
            rooms,
            items: wf.items.into_iter().map(|i| (i.id.clone(), i)).collect(),
            npcs: wf.npcs.into_iter().map(|n| (n.id.clone(), n)).collect(),
            monsters,
            quests: wf.quests.into_iter().map(|q| (q.id.clone(), q)).collect(),
            recipes: wf.recipes.into_iter().map(|r| (r.id.clone(), r)).collect(),
        })
    }

    pub fn room(&self, id: u32) -> Option<&Room> {
        self.rooms.get(&id)
    }

    pub fn room_mut(&mut self, id: u32) -> Option<&mut Room> {
        self.rooms.get_mut(&id)
    }

    /// Cherche une recette par id ou début de nom (insensible à la casse).
    pub fn find_recipe(&self, query: &str) -> Option<&Recipe> {
        let needle = query.to_lowercase();
        self.recipes
            .values()
            .find(|r| r.id.to_lowercase() == needle || r.name.to_lowercase().starts_with(&needle))
    }
}
