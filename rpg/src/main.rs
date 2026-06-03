//! Point d'entrée du jeu.

mod commands;
mod combat;
mod entity;
mod game;
mod magic;
mod player;
mod quest;
mod world;

use game::Game;

fn main() {
    println!("=== RPG Textuel en Rust ===");
    let mut game = match Game::new() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Erreur lors du chargement du monde : {}", e);
            std::process::exit(1);
        }
    };
    game.run();
}
