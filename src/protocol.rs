use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Messages envoyés par le serveur ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMsg {
    /// Premier message reçu après connexion.
    Hello {
        agent_id: Uuid,
        tick_ms: u64,
    },

    /// Challenge de minage : trouver un nonce dont le hash a `target_bits` bits de tête à zéro.
    PowChallenge {
        tick: u64,
        seed: String,
        resource_id: Uuid,
        x: u16,
        y: u16,
        target_bits: u8,
        expires_at: u64,
        value: u32,
    },

    /// Résultat d'un minage : un agent a résolu le challenge.
    PowResult {
        resource_id: Uuid,
        winner: Uuid,
    },

    /// Snapshot de l'état du jeu, envoyé à chaque tick.
    ///   - agents : (id, name, team, score, x, y)
    ///   - resources : (id, x, y, expires_at, value)
    State {
        tick: u64,
        width: u16,
        height: u16,
        goal: u32,
        obstacles: Vec<(u16, u16)>,
        resources: Vec<(Uuid, u16, u16, u64, u32)>,
        agents: Vec<(Uuid, String, String, u32, u16, u16)>,
    },

    /// Un agent commence ou arrête de miner une ressource.
    Mining {
        agent_id: Uuid,
        resource_id: Uuid,
        on: bool,
    },

    /// Une équipe a atteint le score objectif.
    Win {
        team: String,
    },

    /// Erreur envoyée par le serveur.
    Error {
        message: String,
    },
}

// ─── Messages envoyés par le client ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientMsg {
    /// S'enregistrer dans l'arène avec un nom d'équipe et un nom d'agent.
    Register {
        team: String,
        name: String,
    },

    /// Soumettre une solution de minage (nonce trouvé).
    PowSubmit {
        tick: u64,
        resource_id: Uuid,
        nonce: u64,
    },

    /// Heartbeat pour maintenir la connexion.
    Heartbeat {
        tick: u64,
    },

    /// Se déplacer d'une case (dx, dy ∈ {-1, 0, 1}).
    Move {
        dx: i8,
        dy: i8,
    },

    /// Signaler au serveur qu'on commence/arrête de miner.
    Mining {
        resource_id: Uuid,
        on: bool,
    },
}
