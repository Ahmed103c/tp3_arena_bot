// ─── Partie 3 : Stratégie de déplacement ─────────────────────────────────────
//
// Objectif : définir un trait Strategy et l'utiliser via Box<dyn Strategy>
// pour choisir le prochain mouvement du bot à chaque tick.
//
// Concepts exercés : dyn Trait, Box<dyn Strategy>, Send, dispatch dynamique.
//
// ─────────────────────────────────────────────────────────────────────────────

// TODO: Importer les types nécessaires de state.rs
use crate::state::GameState;

// TODO: Définir le trait Strategy.
//
// Le trait doit :
//   - Être object-safe (pas de generics dans les méthodes)
//   - Être Send (pour pouvoir être utilisé dans un contexte multi-thread)
//   - Avoir une méthode next_move qui retourne un déplacement optionnel
//
pub trait Strategy: Send {
    /// Décide du prochain mouvement en fonction de l'état du jeu.
    ///
    /// Retourne Some((dx, dy)) avec dx, dy ∈ {-1, 0, 1}, ou None pour rester sur place.
    fn next_move(&self, state: &GameState) -> Option<(i8, i8)>;
}

// TODO: Implémenter NearestResourceStrategy.
//
// Cette stratégie se dirige vers la ressource la plus proche (distance de Manhattan).
//
pub struct NearestResourceStrategy;
//
impl Strategy for NearestResourceStrategy {
    fn next_move(&self, state: &GameState) -> Option<(i8, i8)> {
        
        // min_by_key retourne Option → ? gère le cas None
        let nearest = state.resources.iter().min_by_key(|r| {
            (r.x as i32 - state.position.0 as i32).abs() +
            (r.y as i32 - state.position.1 as i32).abs()
        })?;

        // nearest.x - position.x donne la direction
        // signum() réduit ça à -1, 0 ou 1
        // as i8 convertit le type
        let dx = (nearest.x as i16 - state.position.0 as i16).signum() as i8;
        let dy = (nearest.y as i16 - state.position.1 as i16).signum() as i8;

        Some((dx, dy))
    }
}

// ─── BONUS : Implémenter d'autres stratégies ────────────────────────────────
//
// Exemples :
//   - RandomStrategy : mouvement aléatoire
//   - FleeStrategy : s'éloigne des autres agents
//   - HybridStrategy : combine plusieurs stratégies
//
// Utilisation dans main.rs :
//   let strategy: Box<dyn Strategy> = Box::new(NearestResourceStrategy);
//
// On peut changer de stratégie sans modifier le reste du code grâce au
// dispatch dynamique (Box<dyn Strategy>).
