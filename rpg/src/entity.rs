//! Objets, PNJ et monstres.

use serde::{Deserialize, Serialize};

/// Trait commun à toute entité ayant des points de vie.
#[allow(dead_code)]
pub trait Vivant {
    fn hp(&self) -> i32;
    fn max_hp(&self) -> i32;

    fn est_vivant(&self) -> bool {
        self.hp() > 0
    }

    /// Ratio PV / PV max, borné dans [0.0, 1.0].
    fn pourcentage_vie(&self) -> f32 {
        let max = self.max_hp().max(1) as f32;
        (self.hp() as f32 / max).clamp(0.0, 1.0)
    }
}

/// Trait pour les entités capables de combattre.
#[allow(dead_code)]
pub trait Combattant: Vivant {
    fn attaque(&self) -> i32;
    fn defense(&self) -> i32;
}

/// Trait pour tout objet identifiable du jeu (items, recettes...).
#[allow(dead_code)]
pub trait Objet {
    fn id(&self) -> &str;
    fn nom(&self) -> &str;
    fn valeur(&self) -> u32;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub description: String,
    /// "Consumable", "Weapon", "Armor", "Treasure"
    pub kind: String,
    pub value: u32,
    #[serde(default)]
    pub heal: i32,
    #[serde(default)]
    pub attack_bonus: i32,
    #[serde(default)]
    pub defense_bonus: i32,
}

impl Item {
    /// Prix de revente chez un marchand (50% de la valeur, minimum 1 si valeur > 0).
    pub fn sell_price(&self) -> u32 {
        let half = self.value / 2;
        if self.value > 0 && half == 0 {
            1
        } else {
            half
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Npc {
    pub id: String,
    pub name: String,
    pub dialogue: String,
    #[serde(default)]
    pub shop: Vec<String>,
    /// Id de la quête offerte par ce PNJ, le cas échéant.
    #[serde(default)]
    pub quest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monster {
    pub id: String,
    pub name: String,
    pub hp: i32,
    pub attack: i32,
    pub xp: u32,
    pub gold: u32,
    /// Identifiants d'objets relâchés à la mort (composants de craft).
    #[serde(default)]
    pub drops: Vec<String>,
}

/// Instance d'un monstre dans une salle (avec ses PV courants).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterInstance {
    pub id: String,
    pub hp: i32,
    #[serde(default)]
    pub effects: Vec<StatusEffect>,
}

impl MonsterInstance {
    pub fn from_template(m: &Monster) -> Self {
        MonsterInstance {
            id: m.id.clone(),
            hp: m.hp,
            effects: Vec::new(),
        }
    }
}

/// Type d'effet de statut.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EffectKind {
    Poison,
    Burn,
}

impl EffectKind {
    pub fn label(&self) -> &'static str {
        match self {
            EffectKind::Poison => "Poison",
            EffectKind::Burn => "Brûlure",
        }
    }
}

/// Effet temporaire appliqué à un monstre (DoT).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEffect {
    pub kind: EffectKind,
    pub damage: i32,
    pub turns: u32,
}

/// Applique les effets actifs : retire les expirés, retourne les dégâts cumulés
/// et imprime un message par effet.
pub fn tick_effects(effects: &mut Vec<StatusEffect>, owner_name: &str) -> i32 {
    let mut total = 0;
    let mut applied: Vec<(EffectKind, i32)> = Vec::new();
    for e in effects.iter_mut() {
        total += e.damage;
        applied.push((e.kind.clone(), e.damage));
        e.turns = e.turns.saturating_sub(1);
    }
    effects.retain(|e| e.turns > 0);
    for (k, d) in applied {
        println!("  ({} subit {} dégats de {}.)", owner_name, d, k.label());
    }
    total
}

/// Ingrédient d'une recette : un id d'objet et la quantité requise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ingredient {
    pub item_id: String,
    pub count: u32,
}

/// Recette de fabrication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    pub inputs: Vec<Ingredient>,
    pub output_id: String,
    #[serde(default = "default_output_count")]
    pub output_count: u32,
}

fn default_output_count() -> u32 {
    1
}

/// Résumé court des effets actifs : `[Poison 4t, Brûlure 2t]`.
pub fn effects_summary(effects: &[StatusEffect]) -> String {
    if effects.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = effects
        .iter()
        .map(|e| format!("{} {}t", e.kind.label(), e.turns))
        .collect();
    format!(" [{}]", parts.join(", "))
}

impl Vivant for Monster {
    fn hp(&self) -> i32 {
        self.hp
    }
    fn max_hp(&self) -> i32 {
        self.hp
    }
}

impl Combattant for Monster {
    fn attaque(&self) -> i32 {
        self.attack
    }
    fn defense(&self) -> i32 {
        0
    }
}

impl Vivant for MonsterInstance {
    fn hp(&self) -> i32 {
        self.hp
    }
    /// L'instance ne porte pas le template ; on expose les PV courants
    /// faute de mieux. Utilise `Monster::max_hp` quand le template est dispo.
    fn max_hp(&self) -> i32 {
        self.hp
    }
}

impl Objet for Item {
    fn id(&self) -> &str {
        &self.id
    }
    fn nom(&self) -> &str {
        &self.name
    }
    fn valeur(&self) -> u32 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poison(dmg: i32, turns: u32) -> StatusEffect {
        StatusEffect {
            kind: EffectKind::Poison,
            damage: dmg,
            turns,
        }
    }

    #[test]
    fn from_template_initialise_pv_et_aucun_effet() {
        let m = Monster {
            id: "g".into(),
            name: "Gobelin".into(),
            hp: 20,
            attack: 5,
            xp: 10,
            gold: 5,
            drops: Vec::new(),
        };
        let inst = MonsterInstance::from_template(&m);
        assert_eq!(inst.id, "g");
        assert_eq!(inst.hp, 20);
        assert!(inst.effects.is_empty());
    }

    #[test]
    fn effect_kind_label_francais() {
        assert_eq!(EffectKind::Poison.label(), "Poison");
        assert_eq!(EffectKind::Burn.label(), "Brûlure");
    }

    #[test]
    fn tick_effects_cumule_les_degats() {
        let mut effs = vec![poison(4, 3), poison(2, 2)];
        let total = tick_effects(&mut effs, "Gobelin");
        assert_eq!(total, 6);
        assert_eq!(effs.len(), 2);
        assert_eq!(effs[0].turns, 2);
        assert_eq!(effs[1].turns, 1);
    }

    #[test]
    fn tick_effects_supprime_les_effets_expires() {
        let mut effs = vec![poison(5, 1)];
        let total = tick_effects(&mut effs, "X");
        assert_eq!(total, 5);
        assert!(effs.is_empty());
    }

    #[test]
    fn effects_summary_vide_si_aucun_effet() {
        assert_eq!(effects_summary(&[]), "");
    }

    #[test]
    fn effects_summary_formate_les_effets() {
        let effs = vec![poison(4, 3)];
        assert_eq!(effects_summary(&effs), " [Poison 3t]");
    }

    #[test]
    fn sell_price_moitie_de_la_valeur() {
        let it = Item {
            id: "x".into(),
            name: "X".into(),
            description: "".into(),
            kind: "Weapon".into(),
            value: 50,
            heal: 0,
            attack_bonus: 0,
            defense_bonus: 0,
        };
        assert_eq!(it.sell_price(), 25);
    }

    #[test]
    fn sell_price_minimum_un_si_valeur_positive() {
        // value=1 -> 1/2 = 0, mais on garantit au moins 1.
        let it = Item {
            id: "x".into(),
            name: "X".into(),
            description: "".into(),
            kind: "Material".into(),
            value: 1,
            heal: 0,
            attack_bonus: 0,
            defense_bonus: 0,
        };
        assert_eq!(it.sell_price(), 1);
    }

    #[test]
    fn sell_price_zero_si_valeur_zero() {
        let it = Item {
            id: "x".into(),
            name: "X".into(),
            description: "".into(),
            kind: "Material".into(),
            value: 0,
            heal: 0,
            attack_bonus: 0,
            defense_bonus: 0,
        };
        assert_eq!(it.sell_price(), 0);
    }

    fn monster_test(hp: i32, attack: i32) -> Monster {
        Monster {
            id: "test".into(),
            name: "Test".into(),
            hp,
            attack,
            xp: 0,
            gold: 0,
            drops: Vec::new(),
        }
    }

    #[test]
    fn vivant_monster_est_vivant_si_hp_positif() {
        assert!(monster_test(10, 5).est_vivant());
        assert!(!monster_test(0, 5).est_vivant());
        assert!(!monster_test(-3, 5).est_vivant());
    }

    #[test]
    fn vivant_pourcentage_vie_borne_entre_0_et_1() {
        let m = monster_test(10, 0);
        assert!((m.pourcentage_vie() - 1.0).abs() < f32::EPSILON);
        let mut inst = MonsterInstance::from_template(&m);
        inst.hp = 5;
        // max_hp d'une instance = hp courant (best-effort), donc ratio = 1.0
        assert!((inst.pourcentage_vie() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn combattant_monster_expose_attaque_et_defense() {
        let m = monster_test(20, 8);
        assert_eq!(m.attaque(), 8);
        assert_eq!(m.defense(), 0);
    }

    #[test]
    fn objet_item_expose_id_nom_valeur() {
        let it = Item {
            id: "epee".into(),
            name: "Épée".into(),
            description: "".into(),
            kind: "Weapon".into(),
            value: 50,
            heal: 0,
            attack_bonus: 5,
            defense_bonus: 0,
        };
        assert_eq!(it.id(), "epee");
        assert_eq!(it.nom(), "Épée");
        assert_eq!(it.valeur(), 50);
    }

    #[test]
    fn vivant_via_dyn_dispatch() {
        // Vérifie qu'on peut traiter Monster et MonsterInstance polymorphiquement.
        let m = monster_test(15, 3);
        let inst = MonsterInstance::from_template(&m);
        let entites: Vec<&dyn Vivant> = vec![&m, &inst];
        assert!(entites.iter().all(|e| e.est_vivant()));
    }
}
