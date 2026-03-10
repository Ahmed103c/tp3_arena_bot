#[allow(unused_imports)]
use std::collections::HashMap;
#[allow(unused_imports)]
use std::sync::{Arc, Mutex};
#[allow(unused_imports)]
use uuid::Uuid;

#[allow(unused_imports)]
use crate::protocol::ServerMsg;

#[derive(Debug, Clone)]
pub struct ResourceInfo {
    pub resource_id: Uuid,
    pub x: u16,
    pub y: u16,
    pub expires_at: u64,
    pub value: u32,
}

#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub id: Uuid,
    pub name: String,
    pub team: String,
    pub score: u32,
    pub x: u16,
    pub y: u16,
}

pub struct GameState {
    pub agent_id: Uuid,
    pub tick: u64,
    pub position: (u16, u16),
    pub map_size: (u16, u16),
    pub goal: u32,
    pub obstacles: Vec<(u16, u16)>,
    pub resources: Vec<ResourceInfo>,
    pub agents: Vec<AgentInfo>,
    pub team_scores: HashMap<String, u32>,
}

impl GameState {
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

                // Mettre à jour les scores par équipe
                self.team_scores.clear();
                for agent in &self.agents {
                    let entry = self.team_scores.entry(agent.team.clone()).or_insert(0);
                    // on garde le max (plusieurs agents même équipe possible)
                    if agent.score > *entry {
                        *entry = agent.score;
                    }
                }

                if let Some(me) = self.agents.iter().find(|a| a.id == self.agent_id) {
                    self.position = (me.x, me.y);
                }
            }

            ServerMsg::PowResult { resource_id, .. } => {
                self.resources.retain(|r| r.resource_id != *resource_id);
            }

            _ => {}
        }
    }

    /// Vérifie si une case (x, y) est bloquée (obstacle ou agent autre que soi-même).
    pub fn is_blocked(&self, x: u16, y: u16) -> bool {
        // Hors carte
        if x >= self.map_size.0 || y >= self.map_size.1 {
            return true;
        }
        // Obstacle statique
        if self.obstacles.contains(&(x, y)) {
            return true;
        }
        // Autre agent sur la case
        if self.agents.iter().any(|a| a.id != self.agent_id && a.x == x && a.y == y) {
            return true;
        }
        // Ressource sur la case (le README dit que le mouvement est bloqué par les ressources aussi)
        if self.resources.iter().any(|r| r.x == x && r.y == y) {
            return true;
        }
        false
    }
}

pub type SharedState = Arc<Mutex<GameState>>;

pub fn new_shared_state(agent_id: Uuid) -> SharedState {
    Arc::new(Mutex::new(GameState::new(agent_id)))
}