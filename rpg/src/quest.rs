//! Système de quêtes : objectifs, progression, récompenses.

use serde::{Deserialize, Serialize};

/// Objectif d'une quête.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Objective {
    /// Tuer N monstres d'un id donné.
    Kill { monster_id: String, count: u32 },
    /// Ramasser/posséder N objets d'un id donné.
    Collect { item_id: String, count: u32 },
}

impl Objective {
    pub fn target_count(&self) -> u32 {
        match self {
            Objective::Kill { count, .. } => *count,
            Objective::Collect { count, .. } => *count,
        }
    }
}

/// Définition d'une quête (chargée depuis JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub objective: Objective,
    pub reward_gold: u32,
    pub reward_xp: u32,
}

/// Etat d'une quête côté joueur.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestProgress {
    pub id: String,
    pub progress: u32,
    pub done: bool,
}

impl QuestProgress {
    pub fn new(id: &str) -> Self {
        QuestProgress {
            id: id.to_string(),
            progress: 0,
            done: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_count_kill() {
        let o = Objective::Kill {
            monster_id: "gobelin".into(),
            count: 3,
        };
        assert_eq!(o.target_count(), 3);
    }

    #[test]
    fn target_count_collect() {
        let o = Objective::Collect {
            item_id: "potion".into(),
            count: 5,
        };
        assert_eq!(o.target_count(), 5);
    }

    #[test]
    fn quest_progress_demarre_a_zero_non_terminee() {
        let qp = QuestProgress::new("chasse_gobelins");
        assert_eq!(qp.id, "chasse_gobelins");
        assert_eq!(qp.progress, 0);
        assert!(!qp.done);
    }

    #[test]
    fn objective_serialise_avec_tag_type() {
        let o = Objective::Kill {
            monster_id: "g".into(),
            count: 2,
        };
        let s = serde_json::to_string(&o).expect("serde");
        assert!(s.contains("\"type\""));
        assert!(s.contains("Kill"));
        let back: Objective = serde_json::from_str(&s).expect("désérialise");
        assert_eq!(back.target_count(), 2);
    }
}
