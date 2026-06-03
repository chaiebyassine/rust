# rust_rpg

## RPG textuel en Rust

Petit jeu de rôle textuel modulaire écrit en Rust, à but pédagogique.

### Caractéristiques

- Personnage avec statistiques (force, intelligence, PV, niveau, XP, or)
- Monde composé de plusieurs salles chargées depuis `data/world.json`
- Inventaire et objets (consommables, armes, trésors)
- PNJ : marchand, forgeron, aubergiste
- Combats au tour par tour contre des monstres
- Cycle jour / nuit avec respawn de monstres (simulation autonome)

### Lancer le jeu

Pré-requis : [Rust](https://www.rust-lang.org/tools/install) (rustup).

```powershell
cd rpg
cargo run
```

### Commandes en jeu

| Commande | Description |
| --- | --- |
| `help` | Affiche l'aide |
| `look` | Décrit la salle |
| `status` | Caractéristiques du personnage |
| `inventory` / `inv` | Inventaire |
| `go <dir>` ou `north/south/east/west` | Se déplacer |
| `take <objet>` | Ramasser un objet |
| `use <objet>` | Utiliser/équiper un objet |
| `talk <pnj>` | Parler à un PNJ |
| `shop` | Voir les marchandises |
| `buy <objet>` | Acheter |
| `attack <monstre>` | Combattre |
| `rest` | Dormir à l'auberge (5 or) |
| `cast <sort> [cible]` | Lancer un sort |
| `spells` | Liste des sorts |
| `learn <sort>` | Apprendre un sort (au sorcier, 30 or) |
| `quests` | Journal de quêtes |
| `accept` | Accepter la quête du PNJ présent |
| `save` | Sauvegarder la partie dans `save.json` |
| `load` | Recharger la sauvegarde |
| `quit` | Quitter |

### Architecture (modèle C4 — niveau 3 : composants)

> Diagrammes C4 complets (Contexte / Conteneurs / Composants / Séquence / Données) :
> voir [docs/architecture.md](docs/architecture.md).

```
main.rs        -> point d'entrée
game.rs        -> boucle de jeu + cycle jour/nuit + quêtes
commands.rs    -> parseur de commandes
world.rs       -> chargement JSON, salles
player.rs      -> stats, inventaire, sorts, quêtes joueur
entity.rs      -> Item, Npc, Monster, MonsterInstance
combat.rs      -> combat au tour par tour
magic.rs       -> sorts (cast/heal)
quest.rs       -> objectifs et progression de quêtes
data/world.json -> données du monde (salles, objets, PNJ, monstres, quêtes)
```

### Licence

GNU GPLv3 — voir `LICENSE`.
