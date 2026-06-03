//! Système de combat tour par tour.

use rand::Rng;

use crate::entity::{tick_effects, Monster, MonsterInstance};
use crate::player::{bar, Player};

/// Déroule un combat. L'instance contient les PV courants ; `tpl` les stats fixes.
/// Retourne true si le joueur a gagné.
pub fn fight(player: &mut Player, instance: &mut MonsterInstance, tpl: &Monster) -> bool {
    println!(
        "\n>> Un {} sauvage apparait ! ({} PV, attaque {})",
        tpl.name, instance.hp, tpl.attack
    );
    let mut rng = rand::thread_rng();

    while player.stats.is_alive() && instance.hp > 0 {
        // Effets de statut sur le monstre (poison, brûlure...).
        let dot = tick_effects(&mut instance.effects, &tpl.name);
        if dot > 0 {
            instance.hp -= dot;
            if instance.hp <= 0 {
                println!(
                    ">> {} succombe à ses afflictions ! +{} XP, +{} or.",
                    tpl.name, tpl.xp, tpl.gold
                );
                player.gold += tpl.gold;
                player.stats.gain_xp(tpl.xp);
                return true;
            }
        }

        // Le joueur attaque.
        let mut damage = player.attack_damage() + rng.gen_range(0..=3);
        let crit = rng.gen_range(0..100) < 10;
        if crit {
            damage *= 2;
        }
        instance.hp -= damage;
        let crit_tag = if crit { " *CRITIQUE !*" } else { "" };
        println!(
            "Tu frappes {} pour {} dégats{}. {} {}/{}",
            tpl.name,
            damage,
            crit_tag,
            bar(instance.hp, tpl.hp, 15),
            instance.hp.max(0),
            tpl.hp,
        );

        if instance.hp <= 0 {
            println!(
                ">> Tu as vaincu {} ! +{} XP, +{} or.",
                tpl.name, tpl.xp, tpl.gold
            );
            player.gold += tpl.gold;
            player.stats.gain_xp(tpl.xp);
            return true;
        }

        // Le monstre riposte.
        let raw = tpl.attack + rng.gen_range(0..=2);
        let mdmg = (raw - player.defense()).max(1);
        player.stats.hp -= mdmg;
        let suffix = if player.defense() > 0 {
            format!(" (armure -{})", player.defense())
        } else {
            String::new()
        };
        println!(
            "{} te blesse de {} dégats{}. {} {}/{}",
            tpl.name,
            mdmg,
            suffix,
            bar(player.stats.hp, player.stats.max_hp, 15),
            player.stats.hp.max(0),
            player.stats.max_hp,
        );
    }

    if !player.stats.is_alive() {
        println!(">> Tu es mort... Game over.");
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EffectKind, StatusEffect};

    fn gobelin() -> Monster {
        Monster {
            id: "gobelin".into(),
            name: "Gobelin".into(),
            hp: 20,
            attack: 5,
            xp: 10,
            gold: 5,
            drops: Vec::new(),
        }
    }

    #[test]
    fn fight_joueur_fort_gagne_et_recupere_recompenses() {
        let mut p = Player::new("Heros".into());
        p.stats.force = 200; // assure la victoire en 1 coup
        let tpl = gobelin();
        let mut inst = MonsterInstance::from_template(&tpl);
        let gold_avant = p.gold;
        let won = fight(&mut p, &mut inst, &tpl);
        assert!(won);
        assert_eq!(p.gold, gold_avant + tpl.gold);
        assert!(p.stats.xp > 0 || p.stats.level > 1);
    }

    #[test]
    fn fight_poison_seul_peut_tuer_le_monstre() {
        // joueur incapable de blesser, mais le poison fait 100 / tour pendant 5 tours.
        let mut p = Player::new("T".into());
        p.stats.force = 0;
        p.equipped_weapon = None;
        let tpl = gobelin();
        let mut inst = MonsterInstance::from_template(&tpl);
        inst.effects.push(StatusEffect {
            kind: EffectKind::Poison,
            damage: 100,
            turns: 1,
        });
        let won = fight(&mut p, &mut inst, &tpl);
        assert!(won, "le poison doit suffire à tuer le monstre");
        assert!(inst.hp <= 0);
    }

    #[test]
    fn fight_joueur_meurt_retourne_false() {
        let mut p = Player::new("Faible".into());
        p.stats.hp = 1;
        p.stats.max_hp = 1;
        p.stats.force = 0;
        // monstre très solide pour être sûr que le joueur meurt avant.
        let tpl = Monster {
            id: "dragon".into(),
            name: "Dragon".into(),
            hp: 9999,
            attack: 999,
            xp: 0,
            gold: 0,
            drops: Vec::new(),
        };
        let mut inst = MonsterInstance::from_template(&tpl);
        let won = fight(&mut p, &mut inst, &tpl);
        assert!(!won);
        assert!(!p.stats.is_alive());
    }
}
