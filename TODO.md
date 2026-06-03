# TODO

## Fait
- Architecture modulaire (player, world, entity, combat, game, commands)
- Caractéristiques, inventaire, or, niveau/XP
- Plusieurs salles, objets, PNJ, monstres chargés depuis world.json
- Combat tour par tour
- Marchand (shop / buy), forgeron, aubergiste (rest)
- Cycle jour/nuit + respawn de monstres la nuit (simulation)
- Sauvegarde / chargement (`save` / `load`, fichier `save.json`)
- Mana + sorts (boule de feu, soin, eclair) + sorcier qui enseigne
- Tour du mage (5ème zone), monstre Loup
- Quêtes scénarisées (Kill / Collect) + journal `quests` / `accept`
- Tests unitaires (8 tests sur parser et joueur)

## Pistes d'amélioration
- Plus de zones et d'ennemis variés
- Gestion d'erreurs : remplacer `unwrap` par `Result` propre
- Diagrammes C4 niveaux 1 et 2 dans `docs/`
