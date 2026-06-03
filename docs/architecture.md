# Architecture du projet — Modélisation C4

Ce document présente l'architecture du jeu **RPG Textuel en Rust** selon le modèle **C4**
(Context / Containers / Components), proposé par Simon Brown.

Les diagrammes sont écrits en **Mermaid** : ils s'affichent automatiquement sur GitHub /
VS Code et peuvent être exportés en image.

---

## 1. Contexte (Niveau 1 — System Context)

Vue d'ensemble : qui utilise le système et avec quoi il interagit.

```mermaid
flowchart TB
    user(["👤 Joueur<br/>(humain, ligne de commande)"])
    rpg["🎮 RPG Textuel<br/>(application Rust)"]
    save[("💾 Fichier de<br/>sauvegarde<br/>save.json")]
    world[("📜 Données du monde<br/>data/world.json")]

    user -- "tape des commandes<br/>(go, attack, cast...)" --> rpg
    rpg -- "affiche le récit,<br/>combats, quêtes" --> user
    rpg -- "lit / écrit la partie" --> save
    rpg -- "charge le monde<br/>(salles, PNJ, monstres,<br/>objets, quêtes)" --> world
```

**Acteurs** :
- **Joueur** : humain qui joue depuis un terminal.
- **RPG Textuel** : le programme Rust, cœur du système.
- **Fichier de sauvegarde** (`save.json`) : persistance d'une partie en cours.
- **Données du monde** (`data/world.json`) : description statique de l'univers.

---

## 2. Conteneurs (Niveau 2 — Containers)

Le projet est une **application monolithique** en Rust : tout tourne dans le même binaire.
On distingue les flux entre l'exécutable et le système de fichiers.

```mermaid
flowchart LR
    subgraph runtime["Application RPG (binaire Rust)"]
        boucle["Boucle principale<br/>(main.rs + game.rs)"]
        parser["Parseur de commandes<br/>(commands.rs)"]
        moteur["Moteur de jeu<br/>(world, player, combat,<br/>magic, quest)"]
    end

    user(["👤 Joueur"])
    fs_save[("save.json")]
    fs_world[("data/world.json")]

    user -- "stdin<br/>(commandes)" --> boucle
    boucle -- "stdout<br/>(narration)" --> user
    boucle --> parser
    parser -- "Command enum" --> moteur
    moteur --> boucle

    moteur -- "serde_json<br/>read/write" --> fs_save
    moteur -- "include_str! +<br/>serde_json parse" --> fs_world
```

**Choix techniques** :
- **Rust 1.96+** (toolchain GNU sous Windows).
- **serde / serde_json** : (dé)sérialisation JSON.
- **rand** : aléa pour le combat, les coups critiques, les sorts AOE.
- **Pas de framework**, pas de réseau, pas de base de données.

---

## 3. Composants (Niveau 3 — Components)

Découpage interne du binaire en modules Rust. Chaque module a une responsabilité claire
(SRP). Les flèches montrent les dépendances `use crate::...`.

```mermaid
flowchart TB
    main["main.rs<br/><i>point d'entrée</i>"]

    subgraph orchestration["Orchestration"]
        game["game.rs<br/>boucle, cycle jour/nuit,<br/>save/load, hooks de quête"]
        commands["commands.rs<br/>parseur de commandes<br/>(enum Command)"]
    end

    subgraph domaine["Domaine métier"]
        player["player.rs<br/>Player, Stats, Class,<br/>équipement, score"]
        world["world.rs<br/>World, Room, chargement<br/>de world.json"]
        entity["entity.rs<br/>Item, Npc, Monster,<br/>MonsterInstance"]
        quest["quest.rs<br/>Quest, Objective,<br/>QuestProgress"]
    end

    subgraph systemes["Systèmes de jeu"]
        combat["combat.rs<br/>fight() — tour par tour,<br/>crits, défense"]
        magic["magic.rs<br/>SPELLS, cast_offensive,<br/>cast_heal, AOE"]
    end

    main --> game
    game --> commands
    game --> world
    game --> player
    game --> combat
    game --> magic
    game --> quest

    combat --> player
    combat --> entity
    magic --> player
    magic --> entity
    player --> entity
    player --> quest
    world --> entity
    world --> quest
```

### Responsabilités détaillées

| Module | Responsabilité |
|--------|----------------|
| `main.rs` | Point d'entrée. Crée et lance `Game::new`. |
| `game.rs` | Boucle de jeu, dispatch des commandes, sauvegarde, cycle jour/nuit, respawn, hooks de quête. |
| `commands.rs` | Parsing du texte saisi par le joueur en variantes typées (`Command::Go(...)`, `Command::Attack(...)`...). |
| `world.rs` | Représentation du monde et chargement initial du JSON. |
| `entity.rs` | Structures de données partagées (`Item`, `Npc`, `Monster`, `MonsterInstance`). |
| `player.rs` | État du personnage : stats, classe, inventaire, équipement, sorts connus, journal de quêtes. |
| `combat.rs` | Algorithme de combat tour par tour (attaque physique, défense, coups critiques). |
| `magic.rs` | Définition statique des sorts, calcul des dégâts magiques, soin, sorts de zone. |
| `quest.rs` | Modèle de quête : objectif (Kill / Collect), progression, état terminé. |

---

## 4. Cycle de vie d'une commande (Diagramme de séquence)

Exemple : le joueur tape `attack gobelin`.

```mermaid
sequenceDiagram
    actor J as Joueur
    participant M as main.rs
    participant G as game.rs
    participant C as commands.rs
    participant W as world.rs
    participant F as combat.rs
    participant P as player.rs

    J->>M: "attack gobelin\n"
    M->>G: read_line(&mut input)
    G->>C: parse(input)
    C-->>G: Command::Attack("gobelin")
    G->>W: room(position)
    W-->>G: &Room (avec MonsterInstance)
    G->>F: fight(player, &mut instance, &template)
    loop tant que joueur et monstre vivants
        F->>P: attack_damage()
        P-->>F: dégâts
        F->>F: roll_crit()
        F-->>J: "Tu frappes Gobelin..."
        F->>P: stats.hp -= dégâts (- défense)
        F-->>J: "Gobelin te blesse..."
    end
    F-->>G: bool (vainqueur ?)
    G->>G: on_kill(monster_id)
    G-->>J: ">> Vainqueur ! +XP, +or"
```

---

## 5. Choix de conception (POO en Rust)

Rust n'a pas d'héritage classique, mais le projet applique les principes objet via :

- **Encapsulation** : chaque struct expose des méthodes (`Player::use_item`,
  `Stats::gain_xp`, `World::load`...).
- **Polymorphisme par enum** :
  - `Command` (enum) modélise toutes les actions du joueur.
  - `Objective` (enum tagué) modélise `Kill` / `Collect`.
  - `Class` (enum) modélise Guerrier / Mage / Voleur.
- **Composition** : `Game` agrège un `World` et un `Player` ; `Player` contient des
  `Stats`, des `Item`, des `QuestProgress`...
- **Sérialisation transparente** : `#[derive(Serialize, Deserialize)]` sur toutes les
  structures persistantes permet `save` / `load` quasi gratuits.

---

## 6. Données externes

`data/world.json` contient :

```mermaid
erDiagram
    WORLD ||--o{ ROOM : contient
    WORLD ||--o{ ITEM : référence
    WORLD ||--o{ NPC : référence
    WORLD ||--o{ MONSTER : référence
    WORLD ||--o{ QUEST : référence
    ROOM }o--o{ ITEM : "items[]"
    ROOM }o--o{ NPC : "npcs[]"
    ROOM }o--o{ MONSTER : "monsters[]"
    NPC }o--|| QUEST : "offre"
    QUEST ||--|| OBJECTIVE : "Kill | Collect"
```

Le format JSON sépare clairement **structure du monde** (rooms) et **catalogue**
(items, npcs, monsters, quests), ce qui rend l'extension du jeu très simple :
ajouter une zone ou un boss = éditer un fichier de données, sans toucher au code.
