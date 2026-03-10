// ─── Partie 3 : Stratégie de déplacement ─────────────────────────────────────

use crate::state::GameState;
use std::collections::{HashSet, VecDeque};

// ─── Trait Strategy ──────────────────────────────────────────────────────────

pub trait Strategy: Send {
    /// Décide du prochain mouvement en fonction de l'état du jeu.
    /// Retourne Some((dx, dy)) avec dx, dy ∈ {-1, 0, 1}, ou None pour rester sur place.
    fn next_move(&self, state: &GameState) -> Option<(i8, i8)>;
}

// ─── NearestResourceStrategy ─────────────────────────────────────────────────
//
// Se dirige vers la ressource la plus proche en utilisant un BFS pour trouver
// le chemin réel (évite les obstacles, les autres agents et les ressources
// bloquantes).
//
// Contraintes du README :
//   - Le mouvement est bloqué si la case cible contient un obstacle,
//     une ressource, ou un autre agent.
//   - On doit être adjacent (distance Manhattan = 1) pour soumettre.
//
pub struct NearestResourceStrategy;

impl Strategy for NearestResourceStrategy {
    fn next_move(&self, state: &GameState) -> Option<(i8, i8)> {
        if state.resources.is_empty() {
            return None;
        }

        // Collecter les positions cibles (cases adjacentes aux ressources)
        // Une ressource bloque le mouvement, donc on veut s'arrêter à côté.
        let targets: HashSet<(u16, u16)> = state
            .resources
            .iter()
            .flat_map(|r| adjacent_cells(r.x, r.y, state.map_size))
            .filter(|&pos| !state.is_blocked(pos.0, pos.1) || pos == state.position)
            .collect();

        if targets.is_empty() {
            // Repli : approche naïve vers la ressource la plus proche
            return naive_direction(state);
        }

        // BFS depuis la position courante vers les cases adjacentes aux ressources
        bfs_first_step(state, &targets)
            .or_else(|| naive_direction(state))
    }
}

/// Retourne les 4 cases adjacentes (haut/bas/gauche/droite) dans les limites de la carte.
fn adjacent_cells(x: u16, y: u16, map_size: (u16, u16)) -> Vec<(u16, u16)> {
    let mut cells = Vec::with_capacity(4);
    if x > 0 { cells.push((x - 1, y)); }
    if x + 1 < map_size.0 { cells.push((x + 1, y)); }
    if y > 0 { cells.push((x, y - 1)); }
    if y + 1 < map_size.1 { cells.push((x, y + 1)); }
    cells
}

/// BFS : retourne le premier pas (dx, dy) vers la cible la plus proche.
/// Respecte les obstacles, agents, et ressources bloquantes.
fn bfs_first_step(state: &GameState, targets: &HashSet<(u16, u16)>) -> Option<(i8, i8)> {
    let start = state.position;

    // Déjà adjacent à une ressource ? Rester sur place (on mine).
    if targets.contains(&start) {
        return None; // None = rester sur place, on est déjà adjacent
    }

    let mut visited: HashSet<(u16, u16)> = HashSet::new();
    // (position, premier_pas_depuis_start)
    let mut queue: VecDeque<((u16, u16), (i8, i8))> = VecDeque::new();

    visited.insert(start);

    // Initialiser la BFS avec les voisins immédiats accessibles
    for (nx, ny) in adjacent_cells(start.0, start.1, state.map_size) {
        if !visited.contains(&(nx, ny)) {
            // La case de départ des voisins : on peut se déplacer vers elle
            // seulement si elle n'est pas bloquée (par un obstacle ou un agent).
            // Note : les ressources bloquent aussi, SAUF si c'est notre cible
            // (cases adjacentes aux ressources, pas les ressources elles-mêmes).
            let blocked = state.obstacles.contains(&(nx, ny))
                || state.agents.iter().any(|a| a.id != state.agent_id && a.x == nx && a.y == ny)
                || state.resources.iter().any(|r| r.x == nx && r.y == ny);

            if !blocked {
                let dx = (nx as i16 - start.0 as i16) as i8;
                let dy = (ny as i16 - start.1 as i16) as i8;
                visited.insert((nx, ny));
                if targets.contains(&(nx, ny)) {
                    return Some((dx, dy));
                }
                queue.push_back(((nx, ny), (dx, dy)));
            }
        }
    }

    // BFS
    while let Some(((cx, cy), first_step)) = queue.pop_front() {
        for (nx, ny) in adjacent_cells(cx, cy, state.map_size) {
            if visited.contains(&(nx, ny)) {
                continue;
            }

            let blocked = state.obstacles.contains(&(nx, ny))
                || state.agents.iter().any(|a| a.id != state.agent_id && a.x == nx && a.y == ny)
                || state.resources.iter().any(|r| r.x == nx && r.y == ny);

            if !blocked {
                visited.insert((nx, ny));
                if targets.contains(&(nx, ny)) {
                    return Some(first_step);
                }
                queue.push_back(((nx, ny), first_step));
            }
        }
    }

    None // aucun chemin trouvé
}

/// Direction naïve (Manhattan) vers la ressource la plus proche.
/// Utilisé comme repli si la BFS échoue (map non initialisée, etc.).
fn naive_direction(state: &GameState) -> Option<(i8, i8)> {
    let nearest = state.resources.iter().min_by_key(|r| {
        (r.x as i32 - state.position.0 as i32).abs()
            + (r.y as i32 - state.position.1 as i32).abs()
    })?;

    let dx = (nearest.x as i16 - state.position.0 as i16).signum() as i8;
    let dy = (nearest.y as i16 - state.position.1 as i16).signum() as i8;

    // Préférer dx d'abord, dy ensuite pour éviter les diagonales
    if dx != 0 {
        Some((dx, 0))
    } else {
        Some((0, dy))
    }
}

// ─── BONUS : Autres stratégies ───────────────────────────────────────────────

/// Stratégie aléatoire : mouvement dans une direction aléatoire accessible.
pub struct RandomStrategy;

impl Strategy for RandomStrategy {
    fn next_move(&self, state: &GameState) -> Option<(i8, i8)> {
        let directions: [(i8, i8); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        let (x, y) = state.position;

        // Filtrer les directions accessibles
        let valid: Vec<(i8, i8)> = directions
            .iter()
            .filter(|&&(dx, dy)| {
                let nx = x as i16 + dx as i16;
                let ny = y as i16 + dy as i16;
                if nx < 0 || ny < 0 { return false; }
                !state.is_blocked(nx as u16, ny as u16)
            })
            .cloned()
            .collect();

        if valid.is_empty() {
            None
        } else {
            // Pseudo-aléatoire via tick pour éviter d'importer rand ici
            let idx = (state.tick as usize) % valid.len();
            Some(valid[idx])
        }
    }
}

/// Stratégie hybride : va vers la ressource la plus proche ET la plus précieuse.
/// Combine distance et valeur pour choisir la meilleure cible.
pub struct ValueWeightedStrategy {
    pub distance_weight: f32,
    pub value_weight: f32,
}

impl Default for ValueWeightedStrategy {
    fn default() -> Self {
        Self { distance_weight: 1.0, value_weight: 2.0 }
    }
}

impl Strategy for ValueWeightedStrategy {
    fn next_move(&self, state: &GameState) -> Option<(i8, i8)> {
        // Choisir la ressource avec le meilleur score (valeur / distance)
        let best = state.resources.iter().min_by(|a, b| {
            let dist_a = ((a.x as i32 - state.position.0 as i32).abs()
                + (a.y as i32 - state.position.1 as i32).abs()) as f32;
            let dist_b = ((b.x as i32 - state.position.0 as i32).abs()
                + (b.y as i32 - state.position.1 as i32).abs()) as f32;

            let score_a = self.value_weight * a.value as f32
                - self.distance_weight * dist_a;
            let score_b = self.value_weight * b.value as f32
                - self.distance_weight * dist_b;

            // min_by → on veut le score MAX donc on inverse
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        })?;

        // Construire un état fictif avec seulement cette ressource pour réutiliser le BFS
        let targets: HashSet<(u16, u16)> = adjacent_cells(best.x, best.y, state.map_size)
            .into_iter()
            .filter(|&pos| !state.is_blocked(pos.0, pos.1) || pos == state.position)
            .collect();

        bfs_first_step(state, &targets).or_else(|| naive_direction(state))
    }
}

/// Stratégie de fuite : s'éloigne des autres agents.
pub struct FleeStrategy;

impl Strategy for FleeStrategy {
    fn next_move(&self, state: &GameState) -> Option<(i8, i8)> {
        let enemies: Vec<&crate::state::AgentInfo> = state
            .agents
            .iter()
            .filter(|a| a.id != state.agent_id)
            .collect();

        if enemies.is_empty() {
            return None;
        }

        let (x, y) = state.position;
        let directions: [(i8, i8); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

        // Choisir la direction qui maximise la distance minimale aux ennemis
        directions
            .iter()
            .filter(|&&(dx, dy)| {
                let nx = x as i16 + dx as i16;
                let ny = y as i16 + dy as i16;
                if nx < 0 || ny < 0 { return false; }
                !state.is_blocked(nx as u16, ny as u16)
            })
            .max_by_key(|&&(dx, dy)| {
                let nx = (x as i16 + dx as i16) as u16;
                let ny = (y as i16 + dy as i16) as u16;
                enemies.iter().map(|e| {
                    (nx as i32 - e.x as i32).abs() + (ny as i32 - e.y as i32).abs()
                }).min().unwrap_or(0)
            })
            .cloned()
    }
}