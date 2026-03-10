// ─── Partie 1 : État partagé ─────────────────────────────────────────────────
//
// Objectif : définir un GameState protégé par Arc<Mutex<>> pour être partagé
// entre le thread lecteur WS et le thread principal.
//
// Concepts exercés : Arc, Mutex, struct, closures.
//
// ─────────────────────────────────────────────────────────────────────────────

// Ces imports seront utilisés dans votre implémentation.
#[allow(unused_imports)]
use std::collections::HashMap;
#[allow(unused_imports)]
use std::sync::{Arc, Mutex};
#[allow(unused_imports)]
use uuid::Uuid;

#[allow(unused_imports)]
use crate::protocol::ServerMsg;

/// Information sur une ressource (challenge de minage) active sur la carte.
#[derive(Debug, Clone)]
pub struct ResourceInfo {
    pub resource_id: Uuid,
    pub x: u16,
    pub y: u16,
    pub expires_at: u64,
    pub value: u32,
}

/// Information sur un agent visible sur la carte.
#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub id: Uuid,
    pub name: String,
    pub team: String,
    pub score: u32,
    pub x: u16,
    pub y: u16,
}

// TODO: Définir la structure GameState.
//
// Elle doit contenir au minimum :
//   - agent_id: Uuid           → votre identifiant (reçu dans Hello)
//   - tick: u64                → tick courant du serveur
//   - position: (u16, u16)    → votre position (x, y)
//   - map_size: (u16, u16)    → dimensions de la carte (width, height)
//   - goal: u32               → score objectif
//   - obstacles: Vec<(u16, u16)>
//   - resources: Vec<ResourceInfo>
//   - agents: Vec<AgentInfo>
//   - team_scores: HashMap<String, u32>
//
pub struct GameState {
    pub agent_id: Uuid,
    pub tick: u64,
    pub position: (u16, u16),
    pub map_size: (u16, u16),
    pub goal: u32,
    pub obstacles: Vec<(u16, u16)>,
    pub resources: Vec<ResourceInfo>,
    pub agents: Vec<AgentInfo>,
    pub team_scores: HashMap<String, u32>
}

// TODO: Implémenter GameState.
//
impl GameState {
    /// Crée un état initial avec l'agent_id reçu du serveur.
    pub fn new(agent_id: Uuid) -> Self {
        GameState {
            agent_id,
            tick: 0,
            position: (0, 0),
            map_size: (0, 0),
            goal: 0,
            obstacles: Vec::new(),
            resources: Vec::new(),
            agents: Vec::new(),
            team_scores: HashMap::new(),
        }
    }

    /// Met à jour l'état à partir d'un message serveur.
    ///
    /// Doit gérer au minimum :
    ///   - ServerMsg::State { .. } → mettre à jour tick, position, resources, agents, etc.
    ///     Indice : votre position est dans la liste `agents`, trouvez-la par agent_id.
    ///   - ServerMsg::PowResult { resource_id, .. } → retirer la ressource de la liste.
    ///
    /// Les autres messages peuvent être ignorés ici.
    pub fn update(&mut self, msg: &ServerMsg) {
        match msg {
            ServerMsg::State { tick, width, height, goal, obstacles, resources, agents } => {
                self.tick = *tick;
                self.map_size = (*width, *height);
                self.goal = *goal;
                self.obstacles = obstacles.clone();

                self.resources = resources.iter().map(|(id, x, y, expires_at, value)| {
                    ResourceInfo {
                        resource_id: *id,
                        x: *x,
                        y: *y,
                        expires_at: *expires_at,
                        value: *value,
                    }
                }).collect();

                self.agents = agents.iter().map(|(id, name, team, score, x, y)| {
                    AgentInfo { 
                        id: *id,
                        name: name.clone(),
                        team: team.clone(),
                        score: *score,
                        x: *x,
                        y: *y,
                    }
                }).collect();

                //trouver la position de mon agent dans self.agents par agent_id
                if let Some(me) = self.agents.iter().find(|a| a.id == self.agent_id) {
                    self.position = (me.x, me.y);
                }
            }

            ServerMsg::PowResult { resource_id, .. } => {
                // .. = ignorer winner
                self.resources.retain(|r| r.resource_id != *resource_id);
            }

            _ => {} // ignorer les autres messages

        }

    }

}

// TODO: Définir le type alias SharedState.
//
// C'est un Arc<Mutex<GameState>> pour pouvoir le partager entre threads.
//
pub type SharedState = Arc<Mutex<GameState>>;
//
// Ajoutez une fonction de construction pratique :
//
pub fn new_shared_state(agent_id: Uuid) -> SharedState {
    Arc::new(Mutex::new(GameState::new(agent_id)))
}
