// ─── Partie 2 : Pool de mineurs ──────────────────────────────────────────────

use std::sync::{Arc, Mutex};
use std::thread;

use uuid::Uuid;

use crate::pow;

/// Requête de minage envoyée aux threads mineurs.
#[derive(Debug, Clone)]
pub struct MineRequest {
    pub seed: String,
    pub tick: u64,
    pub resource_id: Uuid,
    pub agent_id: Uuid,
    pub target_bits: u8,
}

/// Résultat renvoyé par un mineur quand il trouve un nonce valide.
#[derive(Debug, Clone)]
pub struct MineResult {
    pub tick: u64,
    pub resource_id: Uuid,
    pub nonce: u64,
}

pub struct MinerPool {
    pub sender: std::sync::mpsc::Sender<MineRequest>,
    pub receiver: std::sync::mpsc::Receiver<MineResult>,
    /// IDs des ressources actuellement annulées (déjà résolues ou expirées).
    cancelled: Arc<Mutex<std::collections::HashSet<Uuid>>>,
}

impl MinerPool {
    /// Crée un pool de `n` threads mineurs.
    ///
    /// Correction clé : le verrou sur le Receiver est relâché AVANT d'appeler
    /// pow_search(), pour que les autres threads puissent recevoir de nouveaux
    /// challenges pendant qu'un thread mine.
    pub fn new(n: usize) -> Self {
        let (request_tx, request_rx) = std::sync::mpsc::channel::<MineRequest>();
        let (result_tx, result_rx) = std::sync::mpsc::channel::<MineResult>();

        let shared_rx = Arc::new(Mutex::new(request_rx));
        let cancelled: Arc<Mutex<std::collections::HashSet<Uuid>>> =
            Arc::new(Mutex::new(std::collections::HashSet::new()));

        for _ in 0..n {
            let rx = Arc::clone(&shared_rx);
            let tx = result_tx.clone();
            let cancelled = Arc::clone(&cancelled);

            thread::spawn(move || {
                loop {
                    // ── a) Récupérer un challenge (bloquant) ───────────────
                    // IMPORTANT : on relâche le verrou immédiatement après recv()
                    // pour ne pas bloquer les autres threads pendant le minage.
                    let request = match rx.lock().unwrap().recv() {
                        Ok(r) => r,
                        Err(_) => break, // channel fermé → fin du thread
                    };
                    // verrou relâché ici ↑

                    // ── b) Chercher le nonce par batches ───────────────────
                    loop {
                        // Vérifier si cette ressource a été annulée
                        if cancelled.lock().unwrap().contains(&request.resource_id) {
                            break;
                        }

                        let start_nonce = rand::random::<u64>();
                        let found = pow::pow_search(
                            &request.seed,
                            request.tick,
                            request.resource_id,
                            request.agent_id,
                            request.target_bits,
                            start_nonce,
                            100_000,
                        );

                        if let Some(nonce) = found {
                            // Annuler les autres threads qui minent la même ressource
                            cancelled.lock().unwrap().insert(request.resource_id);
                            // Envoyer le résultat (ignorer l'erreur si le channel est fermé)
                            let _ = tx.send(MineResult {
                                tick: request.tick,
                                resource_id: request.resource_id,
                                nonce,
                            });
                            break;
                        }
                    }
                }
            });
        }

        MinerPool {
            sender: request_tx,
            receiver: result_rx,
            cancelled,
        }
    }

    /// Envoie un challenge de minage au pool.
    pub fn submit(&self, request: MineRequest) {
        // Retirer de la liste des annulations au cas où cette ressource
        // aurait été annulée lors d'un tick précédent.
        self.cancelled.lock().unwrap().remove(&request.resource_id);
        self.sender.send(request).unwrap();
    }

    /// Annule le minage d'une ressource (ex : PowResult reçu, quelqu'un d'autre a gagné).
    pub fn cancel(&self, resource_id: Uuid) {
        self.cancelled.lock().unwrap().insert(resource_id);
    }

    /// Tente de récupérer un résultat sans bloquer.
    pub fn try_recv(&self) -> Option<MineResult> {
        self.receiver.try_recv().ok()
    }
}