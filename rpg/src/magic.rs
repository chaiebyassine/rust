//! Système de sorts. Utilise la statistique d'intelligence.

use rand::Rng;

use crate::entity::{EffectKind, Monster, MonsterInstance, StatusEffect};
use crate::player::Player;

/// Description statique d'un sort.
pub struct Spell {
    pub id: &'static str,
    pub name: &'static str,
    pub mana_cost: i32,
    /// Multiplicateur appliqué à l'intelligence pour les dégâts.
    pub power: i32,
    /// Soin (utilisé si > 0, à la place de l'attaque).
    pub heal: i32,
    /// True = sort de zone (frappe tous les monstres présents).
    pub aoe: bool,
    /// Effet appliqué après les dégâts ("poison", "burn", "" si aucun).
    pub effect_kind: &'static str,
    pub effect_dmg: i32,
    pub effect_turns: u32,
}

pub const SPELLS: &[Spell] = &[
    Spell {
        id: "boule_de_feu",
        name: "Boule de feu",
        mana_cost: 10,
        power: 3,
        heal: 0,
        aoe: false,
        effect_kind: "",
        effect_dmg: 0,
        effect_turns: 0,
    },
    Spell {
        id: "soin",
        name: "Soin",
        mana_cost: 8,
        power: 0,
        heal: 25,
        aoe: false,
        effect_kind: "",
        effect_dmg: 0,
        effect_turns: 0,
    },
    Spell {
        id: "eclair",
        name: "Éclair",
        mana_cost: 20,
        power: 6,
        heal: 0,
        aoe: false,
        effect_kind: "",
        effect_dmg: 0,
        effect_turns: 0,
    },
    Spell {
        id: "tempete",
        name: "Tempête",
        mana_cost: 35,
        power: 4,
        heal: 0,
        aoe: true,
        effect_kind: "",
        effect_dmg: 0,
        effect_turns: 0,
    },
    Spell {
        id: "venin",
        name: "Venin",
        mana_cost: 12,
        power: 1,
        heal: 0,
        aoe: false,
        effect_kind: "poison",
        effect_dmg: 4,
        effect_turns: 5,
    },
    Spell {
        id: "incendie",
        name: "Incendie",
        mana_cost: 18,
        power: 2,
        heal: 0,
        aoe: false,
        effect_kind: "burn",
        effect_dmg: 6,
        effect_turns: 3,
    },
];

pub fn find_spell(id_or_name: &str) -> Option<&'static Spell> {
    let needle = id_or_name.to_lowercase();
    SPELLS.iter().find(|s| {
        s.id == needle
            || s.id.starts_with(&needle)
            || s.name.to_lowercase().starts_with(&needle)
    })
}

/// Convertit le tag textuel d'effet en variant typé.
pub fn parse_effect_kind(tag: &str) -> Option<EffectKind> {
    match tag {
        "poison" => Some(EffectKind::Poison),
        "burn" => Some(EffectKind::Burn),
        _ => None,
    }
}

/// Calcule les dégâts d'un sort offensif (selon intelligence et power).
pub fn spell_damage(player: &Player, spell: &Spell) -> i32 {
    (player.stats.intelligence as i32) * spell.power / 2 + 5
}

/// Tire un coup critique (10% de chance). Retourne (damage_final, est_critique).
pub fn roll_crit(damage: i32) -> (i32, bool) {
    let mut rng = rand::thread_rng();
    if rng.gen_range(0..100) < 10 {
        (damage * 2, true)
    } else {
        (damage, false)
    }
}

/// Lance un sort offensif sur un monstre. Retourne true si le monstre est mort.
pub fn cast_offensive(
    player: &mut Player,
    instance: &mut MonsterInstance,
    tpl: &Monster,
    spell: &Spell,
) -> bool {
    let base = spell_damage(player, spell);
    let (damage, crit) = roll_crit(base);
    instance.hp -= damage;
    let crit_tag = if crit { " *CRITIQUE !*" } else { "" };
    println!(
        "Tu lances {} sur {} pour {} dégats magiques{}. ({} PV restants)",
        spell.name,
        tpl.name,
        damage,
        crit_tag,
        instance.hp.max(0)
    );

    // Application d'un éventuel effet de statut.
    if let Some(kind) = parse_effect_kind(spell.effect_kind) {
        if instance.hp > 0 && spell.effect_dmg > 0 && spell.effect_turns > 0 {
            instance.effects.push(StatusEffect {
                kind: kind.clone(),
                damage: spell.effect_dmg,
                turns: spell.effect_turns,
            });
            println!(
                "  >> {} est affecté par {} ({} dégats / tour pendant {} tours).",
                tpl.name,
                kind.label(),
                spell.effect_dmg,
                spell.effect_turns,
            );
        }
    }
    if instance.hp <= 0 {
        println!(
            ">> {} est terrassé par ta magie ! +{} XP, +{} or.",
            tpl.name, tpl.xp, tpl.gold
        );
        player.gold += tpl.gold;
        player.stats.gain_xp(tpl.xp);
        return true;
    }
    false
}

/// Sort de soin sur soi-même.
pub fn cast_heal(player: &mut Player, spell: &Spell) {
    let new_hp = (player.stats.hp + spell.heal).min(player.stats.max_hp);
    let gained = new_hp - player.stats.hp;
    player.stats.hp = new_hp;
    println!("Tu lances {} et récupères {} PV.", spell.name, gained);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_monster() -> Monster {
        Monster {
            id: "gobelin".into(),
            name: "Gobelin".into(),
            hp: 50,
            attack: 5,
            xp: 10,
            gold: 5,
            drops: Vec::new(),
        }
    }

    #[test]
    fn find_spell_par_id_et_nom() {
        assert!(find_spell("boule_de_feu").is_some());
        assert!(find_spell("BOULE").is_some());
        assert!(find_spell("soin").is_some());
        assert!(find_spell("inexistant").is_none());
    }

    #[test]
    fn parse_effect_kind_reconnait_les_tags() {
        assert_eq!(parse_effect_kind("poison"), Some(EffectKind::Poison));
        assert_eq!(parse_effect_kind("burn"), Some(EffectKind::Burn));
        assert!(parse_effect_kind("").is_none());
        assert!(parse_effect_kind("autre").is_none());
    }

    #[test]
    fn spell_damage_depend_intelligence_et_power() {
        let mut p = Player::new("T".into());
        p.stats.intelligence = 10;
        let spell = find_spell("boule_de_feu").unwrap(); // power 3
        // 10 * 3 / 2 + 5 = 20
        assert_eq!(spell_damage(&p, spell), 20);
    }

    #[test]
    fn cast_heal_clamp_au_max_hp() {
        let mut p = Player::new("T".into());
        p.stats.max_hp = 100;
        p.stats.hp = 90;
        let soin = find_spell("soin").unwrap();
        cast_heal(&mut p, soin);
        assert_eq!(p.stats.hp, 100);
    }

    #[test]
    fn cast_offensive_applique_effet_si_present() {
        let mut p = Player::new("T".into());
        p.stats.intelligence = 1;
        let tpl = dummy_monster();
        let mut inst = MonsterInstance::from_template(&tpl);
        let venin = find_spell("venin").unwrap();
        let dead = cast_offensive(&mut p, &mut inst, &tpl, venin);
        assert!(!dead, "le monstre ne doit pas mourir des dégats directs");
        assert_eq!(inst.effects.len(), 1);
        assert_eq!(inst.effects[0].kind, EffectKind::Poison);
        assert_eq!(inst.effects[0].damage, 4);
        assert_eq!(inst.effects[0].turns, 5);
    }

    #[test]
    fn cast_offensive_n_applique_pas_d_effet_pour_sort_simple() {
        let mut p = Player::new("T".into());
        p.stats.intelligence = 1;
        let tpl = dummy_monster();
        let mut inst = MonsterInstance::from_template(&tpl);
        let bdf = find_spell("boule_de_feu").unwrap();
        cast_offensive(&mut p, &mut inst, &tpl, bdf);
        assert!(inst.effects.is_empty());
    }

    #[test]
    fn tous_les_sorts_offensifs_avec_effet_ont_un_kind_valide() {
        for s in SPELLS {
            if !s.effect_kind.is_empty() {
                assert!(
                    parse_effect_kind(s.effect_kind).is_some(),
                    "sort {} a un effect_kind invalide: {}",
                    s.id,
                    s.effect_kind
                );
            }
        }
    }
}
